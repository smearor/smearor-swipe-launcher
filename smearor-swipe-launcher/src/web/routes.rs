use crate::instance::InstanceType;
use crate::instance::LauncherInstance;
use crate::web::template::TemplateEngine;
use axum::Json;
use axum::extract::Path;
use axum::extract::State;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::Message;
use axum::http::StatusCode;
use axum::response::Html;
use axum::response::IntoResponse;
use dashmap::DashMap;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use smearor_model_mcp::InvokeToolMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiHtmlString;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::broadcast;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use tracing::error;

/// A message forwarded from the broker to WebSocket clients.
///
/// Carries the topic, sender, and payload as a JSON string so the client
/// can decide how to apply the update.
#[derive(Clone, Serialize)]
pub struct WebUpdate {
    pub instance_id: String,
    pub topic: String,
    pub sender_id: String,
    pub payload: String,
}

/// Manages WebSocket connections per instance.
///
/// Each instance has a `broadcast::Sender<WebUpdate>`. When a broker message
/// is forwarded, it is sent to the matching instance's broadcast channel,
/// which delivers it to all connected WebSocket clients.
pub struct WebSocketManager {
    channels: DashMap<String, broadcast::Sender<WebUpdate>>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self { channels: DashMap::new() }
    }

    /// Register a new instance for WebSocket updates.
    pub fn register_instance(&self, instance_id: &str) {
        let (tx, _rx) = broadcast::channel::<WebUpdate>(64);
        self.channels.insert(instance_id.to_string(), tx);
    }

    /// Unregister an instance.
    pub fn unregister_instance(&self, instance_id: &str) {
        self.channels.remove(instance_id);
    }

    /// Get the broadcast sender for an instance.
    pub fn get_sender(&self, instance_id: &str) -> Option<broadcast::Sender<WebUpdate>> {
        self.channels.get(instance_id).map(|e| e.value().clone())
    }

    /// Forward a WebUpdate to all WebSocket clients of the given instance.
    pub fn broadcast(&self, update: &WebUpdate) {
        if let Some(sender) = self.get_sender(&update.instance_id) {
            let _ = sender.send(update.clone());
        }
    }
}

/// Shared application state for the web server.
///
/// # Safety
///
/// `LauncherInstance` contains raw pointers (`LoadedPlugin` has `*mut c_void`
/// and `*const WidgetPluginVTable`). Access to `instances` is protected by a `Mutex`,
/// and raw pointers are only dereferenced inside `unsafe` blocks with proper
/// lifetime guarantees. The `broker_sender` is `Send + Sync`.
pub struct WebAppState {
    pub instances: Arc<Mutex<HashMap<String, LauncherInstance>>>,
    pub broker_sender: UnboundedSender<FfiEnvelope>,
    pub template_engine: TemplateEngine,
    pub ws_manager: Arc<WebSocketManager>,
}

unsafe impl Send for WebAppState {}
unsafe impl Sync for WebAppState {}

/// GET `/instances/{id}` — serve the composed HTML page for a web instance.
pub async fn serve_instance_page(Path(instance_id): Path<String>, State(state): State<Arc<WebAppState>>) -> impl IntoResponse {
    let widgets_html;
    let orientation;
    let template_path;

    {
        let instances = state.instances.lock();
        let Ok(instances) = instances else {
            return (StatusCode::INTERNAL_SERVER_ERROR, Html::from("Internal error")).into_response();
        };

        let Some(instance) = instances.get(&instance_id) else {
            return (StatusCode::NOT_FOUND, Html::from("Instance not found")).into_response();
        };

        if instance.instance_type != InstanceType::Web {
            return (StatusCode::BAD_REQUEST, Html::from("Instance is not a web instance")).into_response();
        }

        widgets_html = render_all_widgets_html(instance);
        orientation = match instance.config.layout.orientation {
            crate::config::area::orientation::Orientation::Horizontal => "horizontal",
            crate::config::area::orientation::Orientation::Vertical => "vertical",
        };
        template_path = instance.config.launcher.web_template.clone();
    }

    let mut placeholders = HashMap::new();
    placeholders.insert("instance_id".to_string(), instance_id);
    placeholders.insert("widgets".to_string(), widgets_html);
    placeholders.insert("orientation".to_string(), orientation.to_string());

    let html = state.template_engine.load_and_render(template_path.as_deref(), &placeholders);

    (StatusCode::OK, Html::from(html)).into_response()
}

/// POST `/instances/{id}/{plugin_id}/{action}` — invoke a plugin action.
///
/// Sends an `InvokeToolMessage` through the broker to the targeted plugin.
/// The plugin receives the action name and optional payload, and responds
/// via the normal `InvokeToolResponse` mechanism.
///
/// This is a generic mechanism — each plugin decides which actions it supports.
/// For example, the Button plugin accepts `click`, `longpress`, `swipe_up`,
/// `swipe_down`.
#[derive(Deserialize)]
pub struct ActionRequest {
    payload: Option<Value>,
}

#[derive(Serialize)]
pub struct ActionResponse {
    ok: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    widgets_html: Option<String>,
}

pub async fn handle_action(
    Path((instance_id, plugin_id, action)): Path<(String, String, String)>,
    State(state): State<Arc<WebAppState>>,
    Json(request): Json<ActionRequest>,
) -> impl IntoResponse {
    if let Some(ref payload) = request.payload {
        let payload_size = serde_json::to_vec(payload).map(|v| v.len()).unwrap_or(0);
        if payload_size > 4096 {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(ActionResponse {
                    ok: false,
                    message: format!("Payload exceeds 4 KB limit ({} bytes)", payload_size),
                    widgets_html: None,
                }),
            )
                .into_response();
        }
    }

    // plugin_id from the URL is already namespaced (e.g. "config-web:shelly_fan_button")
    // because render_html emits the DashMap key as data-plugin-id.
    let arguments = match &request.payload {
        Some(payload) => serde_json::json!({ "action": action, "payload": payload }),
        None => serde_json::json!({ "action": action }),
    };

    let correlation_id = format!("web:{}:{}:{}", instance_id, plugin_id, uuid_v4_simple());

    let message = InvokeToolMessage::new(&plugin_id, &correlation_id, &arguments.to_string());

    let payload_ptr = Box::into_raw(Box::new(message)) as *mut core::ffi::c_void;
    let envelope = FfiEnvelope {
        sender_id: stabby::string::String::from(format!("web:{}", instance_id).as_str()),
        target_instance_id: stabby::string::String::from(plugin_id.as_str()),
        topic: stabby::string::String::from(InvokeToolMessage::topic()),
        type_id: InvokeToolMessage::TYPE_ID,
        payload: payload_ptr,
        destroy_payload: Some(default_destroy_payload),
        clone_payload: Some(default_clone_payload::<InvokeToolMessage>),
    };

    // Dispatch directly to the instance, mirroring dispatch_macropad_action.
    // This bypasses the MCP registry lookup on mcp.invoke.tool, which would
    // fail because the tool name (namespaced plugin id) doesn't match the
    // registered tool name (e.g. "button_<id>"). The instance's handle_message
    // routes the envelope to the plugin via target_instance_id.
    let instances = state.instances.lock();
    let Ok(instances) = instances else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ActionResponse {
                ok: false,
                message: "Internal error".to_string(),
                widgets_html: None,
            }),
        )
            .into_response();
    };

    let Some(instance) = instances.get(&instance_id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(ActionResponse {
                ok: false,
                message: "Instance not found".to_string(),
                widgets_html: None,
            }),
        )
            .into_response();
    };

    if instance.instance_type != InstanceType::Web {
        return (
            StatusCode::BAD_REQUEST,
            Json(ActionResponse {
                ok: false,
                message: "Instance is not a web instance".to_string(),
                widgets_html: None,
            }),
        )
            .into_response();
    }

    instance.handle_message(envelope);

    debug!("Web action: instance={}, plugin={}, action={}", instance_id, plugin_id, action);

    (
        StatusCode::OK,
        Json(ActionResponse {
            ok: true,
            message: format!("Action '{}' sent", action),
            widgets_html: None,
        }),
    )
        .into_response()
}

/// Generate a simple unique ID for correlation.
fn uuid_v4_simple() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    format!("{}{}", now.as_millis(), now.subsec_nanos())
}

/// GET `/instances` — list all web instances as JSON.
pub async fn list_web_instances(State(state): State<Arc<WebAppState>>) -> impl IntoResponse {
    let instances = state.instances.lock();
    let Ok(instances) = instances else {
        return Json(Vec::<serde_json::Value>::new());
    };

    let list: Vec<serde_json::Value> = instances
        .values()
        .filter(|i| i.instance_type == InstanceType::Web)
        .map(|i| {
            serde_json::json!({
                "instance_id": i.instance_id,
                "instance_type": i.instance_type.as_str(),
            })
        })
        .collect();

    Json(list)
}

/// Collected plugin data needed to call `render_html` without holding
/// a DashMap reference across a `catch_unwind` boundary.
struct PluginRenderInfo {
    plugin_id: String,
    instance_ptr: *mut core::ffi::c_void,
    render_html: unsafe extern "C" fn(
        instance: *mut core::ffi::c_void,
        instance_id: *const u8,
        instance_id_len: usize,
        plugin_id: *const u8,
        plugin_id_len: usize,
    ) -> FfiHtmlString,
}

/// Render HTML fragments for the plugins of the currently visible area.
pub fn render_all_widgets_html(instance: &LauncherInstance) -> String {
    let plugin_manager = &instance.plugin_manager;

    // Get the plugin entries of the currently visible area, in config order.
    let visible_entries = match instance.area_manager.lock() {
        Ok(area_manager) => area_manager.visible_area_plugin_entries(),
        Err(_) => return String::new(),
    };

    // Collect render info for only the visible area's plugins, in order.
    let instance_id = &instance.instance_id;
    let render_infos: Vec<PluginRenderInfo> = visible_entries
        .iter()
        .filter(|entry| !entry.disabled)
        .filter_map(|entry| {
            let namespaced_id = format!("{}:{}", instance_id, entry.id);
            let loaded = plugin_manager.plugins.get(&namespaced_id)?;
            let vtable = loaded.vtable;
            if vtable.is_null() || loaded.instance.is_null() {
                return None;
            }
            let render_html = unsafe { (*vtable).render_html }?;
            Some(PluginRenderInfo {
                plugin_id: namespaced_id,
                instance_ptr: loaded.instance,
                render_html,
            })
        })
        .collect();

    let instance_id_bytes = instance.instance_id.as_bytes();

    let mut fragments = Vec::new();
    for info in render_infos {
        let plugin_id_bytes = info.plugin_id.as_bytes();

        let result = std::panic::catch_unwind(|| {
            let ffi_string: FfiHtmlString = unsafe {
                (info.render_html)(
                    info.instance_ptr,
                    instance_id_bytes.as_ptr(),
                    instance_id_bytes.len(),
                    plugin_id_bytes.as_ptr(),
                    plugin_id_bytes.len(),
                )
            };
            ffi_string.as_str().to_string()
        });

        match result {
            Ok(html) => {
                fragments.push(html);
            }
            Err(_) => {
                error!("Plugin {} panicked during render_html", info.plugin_id);
            }
        }
    }

    fragments.join("\n")
}

/// Render HTML for a single widget by its namespaced plugin ID.
pub fn render_single_widget_html(instance: &LauncherInstance, namespaced_id: &str) -> String {
    let plugin_manager = &instance.plugin_manager;

    let (instance_ptr, render_html) = {
        let loaded = match plugin_manager.plugins.get(namespaced_id) {
            Some(p) => p,
            None => return String::new(),
        };
        let vtable = loaded.vtable;
        if vtable.is_null() || loaded.instance.is_null() {
            return String::new();
        }
        let render_html = unsafe { (*vtable).render_html };
        let Some(render_html) = render_html else {
            return String::new();
        };
        (loaded.instance, render_html)
    };

    let instance_id_bytes = instance.instance_id.as_bytes();
    let plugin_id_bytes = namespaced_id.as_bytes();

    let result = std::panic::catch_unwind(|| {
        let ffi_string: FfiHtmlString = unsafe {
            (render_html)(
                instance_ptr,
                instance_id_bytes.as_ptr(),
                instance_id_bytes.len(),
                plugin_id_bytes.as_ptr(),
                plugin_id_bytes.len(),
            )
        };
        ffi_string.as_str().to_string()
    });

    match result {
        Ok(html) => html,
        Err(_) => {
            error!("Plugin {} panicked during render_html", namespaced_id);
            String::new()
        }
    }
}

/// GET `/instances/{id}/ws` — WebSocket endpoint for real-time updates.
///
/// Upgrades the HTTP connection to a WebSocket and subscribes to the
/// instance's broadcast channel. The client receives `WebUpdate` messages
/// as JSON text frames.
pub async fn handle_websocket(Path(instance_id): Path<String>, ws: WebSocketUpgrade, State(state): State<Arc<WebAppState>>) -> impl IntoResponse {
    {
        let instances = state.instances.lock();
        let Ok(instances) = instances else {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response();
        };

        let Some(instance) = instances.get(&instance_id) else {
            return (StatusCode::NOT_FOUND, "Instance not found").into_response();
        };

        if instance.instance_type != InstanceType::Web {
            return (StatusCode::BAD_REQUEST, "Instance is not a web instance").into_response();
        }
    }

    let sender = match state.ws_manager.get_sender(&instance_id) {
        Some(sender) => sender,
        None => return (StatusCode::NOT_FOUND, "Instance not registered for WebSocket").into_response(),
    };

    let mut rx = sender.subscribe();

    ws.on_upgrade(move |mut socket| async move {
        debug!("WebSocket connected for instance {}", instance_id);

        loop {
            tokio::select! {
                // Outgoing: broadcast updates to client
                result = rx.recv() => {
                    match result {
                        Ok(update) => {
                            let json = match serde_json::to_string(&update) {
                                Ok(j) => j,
                                Err(e) => {
                                    error!("Failed to serialize WebUpdate: {}", e);
                                    continue;
                                }
                            };
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            debug!("WebSocket client lagged by {} messages for instance {}", n, instance_id);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            debug!("WebSocket broadcast channel closed for instance {}", instance_id);
                            break;
                        }
                    }
                }
                // Incoming: action messages from client
                result = socket.recv() => {
                    match result {
                        Some(Ok(Message::Text(text))) => {
                            handle_websocket_action(&text, &instance_id, &state);
                        }
                        Some(Ok(Message::Binary(data))) => {
                            if let Ok(text) = std::str::from_utf8(&data) {
                                handle_websocket_action(text, &instance_id, &state);
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            break;
                        }
                        Some(Ok(_)) => {}
                        Some(Err(e)) => {
                            debug!("WebSocket receive error for instance {}: {}", instance_id, e);
                            break;
                        }
                    }
                }
            }
        }

        debug!("WebSocket disconnected for instance {}", instance_id);
    })
}

/// Handle an incoming WebSocket action message from the client.
///
/// Expected JSON format: `{"plugin_id":"config-web:games_menu_button","action":"click"}`
/// Optionally with `"payload": {...}`.
fn handle_websocket_action(text: &str, instance_id: &str, state: &WebAppState) {
    let msg: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            debug!("WebSocket: failed to parse action message: {}", e);
            return;
        }
    };

    let plugin_id = match msg.get("plugin_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            debug!("WebSocket: action message missing plugin_id");
            return;
        }
    };

    let action = msg.get("action").and_then(|v| v.as_str()).unwrap_or("click");

    let arguments = match msg.get("payload") {
        Some(payload) => serde_json::json!({ "action": action, "payload": payload }),
        None => serde_json::json!({ "action": action }),
    };

    let correlation_id = format!("ws:{}:{}:{}", instance_id, plugin_id, uuid_v4_simple());

    let message = InvokeToolMessage::new(&plugin_id, &correlation_id, &arguments.to_string());

    let payload_ptr = Box::into_raw(Box::new(message)) as *mut core::ffi::c_void;
    let envelope = FfiEnvelope {
        sender_id: stabby::string::String::from(format!("web:{}", instance_id).as_str()),
        target_instance_id: stabby::string::String::from(plugin_id.as_str()),
        topic: stabby::string::String::from(InvokeToolMessage::topic()),
        type_id: InvokeToolMessage::TYPE_ID,
        payload: payload_ptr,
        destroy_payload: Some(default_destroy_payload),
        clone_payload: Some(default_clone_payload::<InvokeToolMessage>),
    };

    let instances = state.instances.lock();
    let Ok(instances) = instances else {
        return;
    };
    let Some(instance) = instances.get(instance_id) else {
        return;
    };

    instance.handle_message(envelope);
    debug!("WebSocket action: instance={}, plugin={}, action={}", instance_id, plugin_id, action);
}

/// Extract a JSON-serializable payload string from an FfiEnvelope.
///
/// For String payloads, returns the string directly.
/// For other payload types, returns a JSON object with type_id for the client
/// to interpret.
pub fn extract_payload_as_json(envelope: &FfiEnvelope) -> String {
    let string_type_id = smearor_swipe_launcher_plugin_api::generate_type_id("std::string::String");

    if envelope.type_id == string_type_id && !envelope.payload.is_null() {
        if let Some(payload) = unsafe { (envelope.payload as *const String).as_ref() } {
            return payload.clone();
        }
    }

    serde_json::json!({
        "type_id": envelope.type_id,
        "note": "non-string payload"
    })
    .to_string()
}
