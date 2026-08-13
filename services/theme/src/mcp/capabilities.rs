use crate::service::ThemeService;
use schemars::schema_for;
use smearor_model_mcp::NoArgs;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_theme_model::SetThemeArgs;

impl McpCapabilitiesRegistrator for ThemeService {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let resource_status = RegisterResourceMessage::new(
            "theme://status",
            "Theme Status",
            "Current theme service status including applied theme, effective mode, and configured themes.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(resource_status);

        let resource_themes = RegisterResourceMessage::new(
            "theme://themes",
            "Theme Themes",
            "List of all configured themes with their metadata and colors.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(resource_themes);

        let no_args_schema = serde_json::to_string(&schema_for!(NoArgs)).unwrap_or_default();
        let tool_get = RegisterToolMessage::new(
            "get_theme",
            "Get the current theme status including applied theme, effective mode, and configured themes.",
            &no_args_schema,
        )
        .with_annotations(&smearor_model_mcp::ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(tool_get);

        let set_schema = serde_json::to_string(&schema_for!(SetThemeArgs)).unwrap_or_default();
        let tool_set = RegisterToolMessage::new("set_theme", "Select and apply a theme by name immediately. This changes the active CSS theme.", &set_schema)
            .with_annotations(&smearor_model_mcp::ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(tool_set);

        let prompt = RegisterPromptMessage::with_memory(
            "theme_guide",
            "Returns a system prompt with theme management tools, resources, and current status snapshot.",
            &no_args_schema,
            "theme preferences and default theme",
            "theme,css,colors,dark,light",
        );
        broadcaster.broadcast_message_to_topic(prompt);
    }
}
