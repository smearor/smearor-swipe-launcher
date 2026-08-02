use crate::service::WallpaperService;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

impl McpCapabilitiesRegistrator for WallpaperService {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let resource_status = RegisterResourceMessage::new(
            "wallpaper://status",
            "Wallpaper Status",
            "Current wallpaper service status including running theme and configured themes.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(resource_status);

        let resource_themes = RegisterResourceMessage::new(
            "wallpaper://themes",
            "Wallpaper Themes",
            "List of all configured wallpaper themes with their configurations.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(resource_themes);

        let tool_add = RegisterToolMessage::new(
            "add_wallpaper_theme",
            "Permanently appends a new wallpaper theme to the configuration store.",
            r#"{ "type": "object", "properties": { "name": { "type": "string" }, "type": { "type": "string", "enum": ["Video", "Image", "Application"] }, "config": { "type": "object" }, "description": { "type": "string" }, "preview_image_path": { "type": "string" }, "preview_icon": { "type": "string" } }, "required": ["name", "type", "config"] }"#,
        );
        broadcaster.broadcast_message_to_topic(tool_add);

        let tool_remove = RegisterToolMessage::new(
            "remove_wallpaper_theme",
            "Deletes a wallpaper theme from the configuration store.",
            r#"{ "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"] }"#,
        );
        broadcaster.broadcast_message_to_topic(tool_remove);

        let tool_select = RegisterToolMessage::new(
            "select_wallpaper_theme",
            "Selects a wallpaper theme by name without starting it. Updates the selected_theme state.",
            r#"{ "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"] }"#,
        );
        broadcaster.broadcast_message_to_topic(tool_select);

        let tool_start = RegisterToolMessage::new(
            "start_selected_wallpaper_process",
            "Starts the currently selected wallpaper theme. Stops any running theme first, then spawns the engine process.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(tool_start);

        let tool_stop = RegisterToolMessage::new(
            "stop_current_wallpaper_process",
            "Stops the currently running wallpaper process immediately.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(tool_stop);
    }
}
