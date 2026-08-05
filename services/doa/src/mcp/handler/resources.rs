use crate::service::DoaService;
use smearor_doa_model::DoaDirectionResponse;
use smearor_doa_model::DoaMcpResources;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use tracing::debug;

impl McpResourceHandler<DoaMcpResources> for DoaService {
    fn get_response(&self, request: &ResourceRequest<DoaMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        let state = self.state_snapshot();

        match request.resource {
            DoaMcpResources::Status => {
                let response_payload = DoaDirectionResponse::from(state);
                let json = serde_json::to_string(&response_payload).unwrap_or_else(|e| {
                    debug!("DoA Service: failed to serialize DoaDirectionResponse: {e}");
                    format!("{{\"error\":\"Serialization failed: {e}\"}}")
                });
                InvokeResourceResponse::success(correlation_id, &json)
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for DoaService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        self.handle_invoke_resource_message(message, sender_id);
    }
}
