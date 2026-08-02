use crate::service::AudioService;
use smearor_audio_model::AudioMcpResources;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl McpResourceHandler<AudioMcpResources> for AudioService {
    fn get_response(&self, request: &ResourceRequest<AudioMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        let Some(status) = self.status_snapshot() else {
            return InvokeResourceResponse::error(correlation_id, "Audio status not yet available");
        };

        match request.resource {
            AudioMcpResources::Status => {
                let json = serde_json::json!({
                    "volume": status.volume,
                    "is_muted": status.is_muted,
                    "active_device": status.active_device.as_ref().map(|d| serde_json::json!({
                        "id": d.id,
                        "name": d.name.to_string(),
                        "is_default": d.is_default,
                    })).unwrap_or(serde_json::Value::Null),
                    "output_devices": status.output_devices.iter().map(|d| serde_json::json!({
                        "id": d.id,
                        "name": d.name.to_string(),
                        "is_default": d.is_default,
                    })).collect::<Vec<_>>(),
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            AudioMcpResources::Volume => {
                let json = serde_json::json!({ "volume": status.volume });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            AudioMcpResources::Muted => {
                let json = serde_json::json!({ "is_muted": status.is_muted });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            AudioMcpResources::ActiveSink => match status.active_device.as_ref() {
                Some(d) => {
                    let json = serde_json::json!({
                        "id": d.id,
                        "name": d.name.to_string(),
                        "is_default": d.is_default,
                    });
                    InvokeResourceResponse::success(correlation_id, &json.to_string())
                }
                None => InvokeResourceResponse::success(correlation_id, "null"),
            },
            AudioMcpResources::Sinks => {
                let devices: Vec<serde_json::Value> = status
                    .output_devices
                    .iter()
                    .map(|d| {
                        serde_json::json!({
                            "id": d.id,
                            "name": d.name.to_string(),
                            "is_default": d.is_default,
                        })
                    })
                    .collect();
                let json = serde_json::Value::Array(devices);
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for AudioService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        self.handle_invoke_resource_message(message, sender_id);
    }
}
