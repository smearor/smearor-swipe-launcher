use crate::service::PersonalizationService;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_personalization_model::PersonalizationMcpResources;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use tracing::trace;

impl McpResourceHandler<PersonalizationMcpResources> for PersonalizationService {
    fn get_response(&self, request: &ResourceRequest<PersonalizationMcpResources>) -> InvokeResourceResponse {
        match request.resource {
            PersonalizationMcpResources::Profile => {
                let state = self.latest_state.read().map(|s| s.clone()).unwrap_or_default();
                let json = serde_json::to_string(&state.status).unwrap_or_else(|_| "{}".to_string());
                InvokeResourceResponse::success(request.correlation_id, &json)
            }
        }
    }

    fn send_resource_response(&self, response: InvokeResourceResponse, sender_id: &str) {
        self.send_response(response, sender_id);
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for PersonalizationService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        trace!("personalization: InvokeResourceMessage handler uri={} sender_id={}", message.0.uri, sender_id);
        self.handle_invoke_resource_message(message, sender_id);
    }
}
