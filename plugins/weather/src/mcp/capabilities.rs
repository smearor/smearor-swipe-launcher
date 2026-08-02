use crate::widget::WeatherWidget;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

impl McpCapabilitiesRegistrator for WeatherWidget {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let resource = RegisterResourceMessage::new(
            "weather://widget",
            "Weather Widget",
            "Current weather data displayed by the weather widget.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(resource);

        let refresh_tool = RegisterToolMessage::new(
            "weather_widget_refresh",
            "Force the weather widget to request a data refresh.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(refresh_tool);

        let button_tool_name = format!("button_{}", self.meta.id);
        let button_tool = RegisterToolMessage::new(
            &button_tool_name,
            "Trigger an action on the weather widget (view switching or input action).",
            r#"{ "type": "object", "properties": { "action": { "type": "string", "enum": ["click", "longpress", "double_press", "swipe_up", "swipe_down", "right_click", "middle_click", "scroll_up", "scroll_down", "expand", "collapse", "toggle_view"], "description": "The action to trigger" } }, "required": ["action"] }"#,
        );
        broadcaster.broadcast_message_to_topic(button_tool);
    }
}
