use crate::widget::ButtonWidget;
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
        let tool = RegisterToolMessage::new(
            &tool_name,
            description,
            r#"{ "type": "object", "properties": { "action": { "type": "string", "enum": ["click", "longpress", "hold_start", "hold_stop", "double_press", "compound_longpress", "swipe_up", "swipe_down", "right_click", "middle_click", "scroll_up", "scroll_down"], "description": "The button action to trigger" } }, "required": ["action"] }"#,
        );
        let broadcaster = self.get_broadcaster();
        broadcaster.broadcast_message_to_topic(tool);
        debug!("ButtonWidget registered MCP tool: {}", tool_name);
    }
}
