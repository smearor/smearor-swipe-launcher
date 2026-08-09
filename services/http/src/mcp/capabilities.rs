use crate::service::HttpService;
use schemars::schema_for;
use smearor_http_model::HttpRequestArgs;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::ToolAnnotations;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

impl McpCapabilitiesRegistrator for HttpService {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let schema = serde_json::to_string(&schema_for!(HttpRequestArgs)).unwrap_or_default();
        let http_request_tool = RegisterToolMessage::new("http_request", "Execute an HTTP request to a whitelisted URL and return the response.", &schema)
            .with_annotations(&ToolAnnotations::read_only().with_open_world(true));
        broadcaster.broadcast_message_to_topic(http_request_tool);
    }
}
