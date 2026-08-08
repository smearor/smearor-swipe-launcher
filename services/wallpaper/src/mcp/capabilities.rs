use crate::service::WallpaperService;
use schemars::schema_for;
use smearor_model_mcp::NoArgs;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_wallpaper_model::AddWallpaperThemeArgs;
use smearor_wallpaper_model::RemoveWallpaperThemeArgs;
use smearor_wallpaper_model::SelectWallpaperThemeArgs;

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

        let add_schema = serde_json::to_string(&schema_for!(AddWallpaperThemeArgs)).unwrap_or_default();
        let tool_add = RegisterToolMessage::new("add_wallpaper_theme", "Permanently appends a new wallpaper theme to the configuration store.", &add_schema);
        broadcaster.broadcast_message_to_topic(tool_add);

        let remove_schema = serde_json::to_string(&schema_for!(RemoveWallpaperThemeArgs)).unwrap_or_default();
        let tool_remove = RegisterToolMessage::new("remove_wallpaper_theme", "Deletes a wallpaper theme from the configuration store.", &remove_schema);
        broadcaster.broadcast_message_to_topic(tool_remove);

        let select_schema = serde_json::to_string(&schema_for!(SelectWallpaperThemeArgs)).unwrap_or_default();
        let tool_select = RegisterToolMessage::new(
            "select_wallpaper_theme",
            "Selects a wallpaper theme by name without starting it. Updates the selected_theme state.",
            &select_schema,
        );
        broadcaster.broadcast_message_to_topic(tool_select);

        let no_args_schema = serde_json::to_string(&schema_for!(NoArgs)).unwrap_or_default();
        let tool_start = RegisterToolMessage::new(
            "start_selected_wallpaper_process",
            "Starts the currently selected wallpaper theme. Stops any running theme first, then spawns the engine process.",
            &no_args_schema,
        );
        broadcaster.broadcast_message_to_topic(tool_start);

        let tool_stop = RegisterToolMessage::new(
            "stop_current_wallpaper_process",
            "Stops the currently running wallpaper process immediately.",
            &no_args_schema,
        );
        broadcaster.broadcast_message_to_topic(tool_stop);

        let prompt = RegisterPromptMessage::with_memory(
            "wallpaper_guide",
            "Returns a system prompt with wallpaper theme management tools, resources, and current status snapshot.",
            &no_args_schema,
            "wallpaper theme preferences and default theme",
            "wallpaper,theme,background",
        );
        broadcaster.broadcast_message_to_topic(prompt);
    }
}
