use crate::service::GnomeWorkspaceService;
use schemars::schema_for;
use smearor_model_compositor::SwitchWorkspaceArgs;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::ToolAnnotations;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

impl McpCapabilitiesRegistrator for GnomeWorkspaceService {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let workspaces_resource = RegisterResourceMessage::new(
            "compositor://workspaces",
            "Workspace Snapshot",
            "Current GNOME workspace layout: all workspaces with names, active state, and monitor assignment.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(workspaces_resource);

        let schema = serde_json::to_string(&schema_for!(SwitchWorkspaceArgs)).unwrap_or_default();
        let switch_workspace_tool =
            RegisterToolMessage::new("compositor_switch_workspace", "Switch to a workspace by ID.", &schema).with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(switch_workspace_tool);
    }
}
