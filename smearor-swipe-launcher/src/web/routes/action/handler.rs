use crate::instance::InstanceType;
use crate::web::action::ActionRequest;
use crate::web::action::ActionResponse;
use crate::web::routes::utils::uuid_v4_simple;
use crate::web::state::WebAppState;
use axum::Json;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use smearor_model_mcp::InvokeToolMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;
use std::sync::Arc;
use tracing::debug;

/// POST `/instances/{id}/{plugin_id}/{action}` — invoke a plugin action.
///
/// Sends an `InvokeToolMessage` through the broker to the targeted plugin.
/// The plugin receives the action name and optional payload, and responds
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
