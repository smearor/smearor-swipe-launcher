use crate::service::AppLauncherService;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use tracing::debug;

impl McpCapabilitiesRegistrator for AppLauncherService {
    fn register_mcp_capabilities(&self) {
        if !self.config.mcp_enabled {
            debug!("App Launcher Service: MCP tool registration disabled by config");
            return;
        }

        let broadcaster = self.get_broadcaster();

        let running_apps_resource = RegisterResourceMessage::new(
            "app_launcher://running_apps",
            "Running Applications",
            "List of currently running tracked applications with their PIDs and termination policy.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(running_apps_resource);

        let available_apps_resource = RegisterResourceMessage::new(
            "app_launcher://available_apps",
            "Available Applications",
            "List of all available .desktop files found in standard application directories, sorted by name.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(available_apps_resource);

        let exec_tool = RegisterToolMessage::new(
            "app_launcher_exec",
            "Launch an application by desktop file path. The desktop file path must be the canonical path to a .desktop file.",
            r#"{ "type": "object", "properties": { "desktop_file": { "type": "string", "description": "Canonical path to the .desktop file" }, "forked": { "type": "boolean", "description": "Whether the process should be detached from the launcher (default: false)" }, "terminate_on_exit": { "type": "boolean", "description": "Whether to terminate the process when the launcher exits (default: true)" } }, "required": ["desktop_file"] }"#,
        );
        broadcaster.broadcast_message_to_topic(exec_tool);

        let terminate_tool = RegisterToolMessage::new(
            "app_launcher_terminate",
            "Terminate a running application by desktop file path.",
            r#"{ "type": "object", "properties": { "desktop_file": { "type": "string", "description": "Canonical path to the .desktop file" } }, "required": ["desktop_file"] }"#,
        );
        broadcaster.broadcast_message_to_topic(terminate_tool);

        let search_tool = RegisterToolMessage::new(
            "app_launcher_search_apps",
            "Search for available applications by name. Returns matching .desktop file paths and names. Use this to find the correct desktop_file path before calling app_launcher_exec.",
            r#"{ "type": "object", "properties": { "query": { "type": "string", "description": "Search query (e.g., 'calculator', 'browser', 'gimp'). Matches against app names case-insensitively." } }, "required": ["query"] }"#,
        );
        broadcaster.broadcast_message_to_topic(search_tool);

        let prompt = RegisterPromptMessage::with_memory(
            "app_launch_guide",
            "System message with app search and launch pipeline instructions.",
            r#"{ "type": "object", "properties": {} }"#,
            "favorite applications and frequently launched apps",
            "app,application",
        );
        broadcaster.broadcast_message_to_topic(prompt);
    }
}
