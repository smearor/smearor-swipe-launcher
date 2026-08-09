use crate::widget::WeatherWidget;
use schemars::schema_for;
use smearor_model_mcp::ButtonActionArgs;
use smearor_model_mcp::NoArgs;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::ToolAnnotations;
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

        let no_args_schema = serde_json::to_string(&schema_for!(NoArgs)).unwrap_or_default();
        let refresh_tool = RegisterToolMessage::new("weather_widget_refresh", "Force the weather widget to request a data refresh.", &no_args_schema)
            .with_annotations(&ToolAnnotations::read_only().with_open_world(true));
        broadcaster.broadcast_message_to_topic(refresh_tool);

        let button_schema = serde_json::to_string(&schema_for!(ButtonActionArgs)).unwrap_or_default();
        let button_tool_name = format!("button_{}", self.meta.id);
        let button_tool = RegisterToolMessage::new(
            &button_tool_name,
            self.config
                .metadata
                .description()
                .unwrap_or("Trigger an action on the weather widget (view switching or input action)."),
            &button_schema,
        )
        .with_annotations(&ToolAnnotations::read_only())
        .maybe_with_title(self.config.metadata.title());
        broadcaster.broadcast_message_to_topic(button_tool);
    }
}
