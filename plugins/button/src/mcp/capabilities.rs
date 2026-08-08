use crate::widget::ButtonWidget;
use schemars::schema_for;
use smearor_model_mcp::ButtonActionArgs;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use tracing::debug;

impl McpCapabilitiesRegistrator for ButtonWidget {
    fn register_mcp_capabilities(&self) {
        let Some(description) = &self.config.description else {
            return;
        };
        let tool_name = format!("button_{}", self.meta.id);
        let schema = serde_json::to_string(&schema_for!(ButtonActionArgs)).unwrap_or_default();
        let tool = RegisterToolMessage::new(&tool_name, description, &schema);
        let broadcaster = self.get_broadcaster();
        broadcaster.broadcast_message_to_topic(tool);
        debug!("ButtonWidget registered MCP tool: {}", tool_name);
    }
}
