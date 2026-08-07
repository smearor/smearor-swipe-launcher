use crate::instance::InstanceType;
use crate::web::routes::utils::uuid_v4_simple;
use crate::web::state::WebAppState;
use axum::extract::Path;
use axum::extract::State;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::Message;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use smearor_model_mcp::InvokeToolMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::box_payload;
use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::debug;
use tracing::error;

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

    let payload_ptr = box_payload(message);
    let envelope = FfiEnvelope::builder()
        .sender_id(format!("web:{}", instance_id))
        .target_instance_id(plugin_id.as_str())
        .topic(InvokeToolMessage::topic())
        .type_id(InvokeToolMessage::TYPE_ID)
        .payload(payload_ptr)
        .destroy_payload(Some(default_destroy_payload))
        .clone_payload(Some(default_clone_payload::<InvokeToolMessage>))
        .build();

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
