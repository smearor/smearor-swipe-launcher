use crate::service::AppLauncherService;
use schemars::schema_for;
use smearor_app_launcher_model::AppLauncherExecArgs;
use smearor_app_launcher_model::AppLauncherSearchAppsArgs;
use smearor_app_launcher_model::AppLauncherTerminateArgs;
use smearor_model_mcp::NoArgs;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::ToolAnnotations;
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

        let exec_schema = serde_json::to_string(&schema_for!(AppLauncherExecArgs)).unwrap_or_default();
        let exec_tool = RegisterToolMessage::new(
            "app_launcher_exec",
            "Launch an application by desktop file path. The desktop file path must be the canonical path to a .desktop file.",
            &exec_schema,
        )
        .with_annotations(&ToolAnnotations::destructive().with_open_world(true));
        broadcaster.broadcast_message_to_topic(exec_tool);

        let terminate_schema = serde_json::to_string(&schema_for!(AppLauncherTerminateArgs)).unwrap_or_default();
        let terminate_tool = RegisterToolMessage::new("app_launcher_terminate", "Terminate a running application by desktop file path.", &terminate_schema)
            .with_annotations(&ToolAnnotations::destructive().with_open_world(true));
        broadcaster.broadcast_message_to_topic(terminate_tool);

        let search_schema = serde_json::to_string(&schema_for!(AppLauncherSearchAppsArgs)).unwrap_or_default();
        let search_tool = RegisterToolMessage::new(
            "app_launcher_search_apps",
            "Search for available applications by name, generic name, comment, keywords, or categories. Returns matching .desktop file paths with full metadata (name, generic_name, comment, keywords, categories). Use this to find the correct desktop_file path before calling app_launcher_exec.",
            &search_schema,
        )
            .with_annotations(&ToolAnnotations::read_only());
        broadcaster.broadcast_message_to_topic(search_tool);

        let no_args_schema = serde_json::to_string(&schema_for!(NoArgs)).unwrap_or_default();
        let prompt = RegisterPromptMessage::with_memory(
            "app_launch_guide",
            "System message with app search and launch pipeline instructions.",
            &no_args_schema,
            "favorite applications and frequently launched apps",
            "app,application",
        );
        broadcaster.broadcast_message_to_topic(prompt);
    }
}
