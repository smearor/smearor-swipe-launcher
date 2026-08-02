use crate::widget::PowerWidget;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

impl McpCapabilitiesRegistrator for PowerWidget {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let button_tool_name = format!("button_{}", self.meta.id);
        let button_tool = RegisterToolMessage::new(
            &button_tool_name,
            "Trigger an action on the power widget (view switching, power action, or input action).",
            r#"{ "type": "object", "properties": { "action": { "type": "string", "enum": ["click", "longpress", "double_press", "swipe_up", "swipe_down", "right_click", "middle_click", "scroll_up", "scroll_down", "expand", "collapse", "toggle_view"], "description": "The action to trigger" } }, "required": ["action"] }"#,
        );
        broadcaster.broadcast_message_to_topic(button_tool);
    }
}
