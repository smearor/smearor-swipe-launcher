use crate::service::HyprlandService;
use schemars::schema_for;
use smearor_hyprland_model::MoveWindowArgs;
use smearor_hyprland_model::SwitchWorkspaceArgs;
use smearor_hyprland_model::ToggleFloatingArgs;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

impl McpCapabilitiesRegistrator for HyprlandService {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let state_resource = RegisterResourceMessage::new(
            "hyprland://state",
            "Hyprland State",
            "Current Hyprland compositor state: active window, fullscreen, keyboard layout, submap.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(state_resource);

        let active_window_resource = RegisterResourceMessage::new(
            "hyprland://active-window",
            "Active Window",
            "Information about the currently focused window (class, title, workspace).",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(active_window_resource);

        let switch_workspace_schema = serde_json::to_string(&schema_for!(SwitchWorkspaceArgs)).unwrap_or_default();
        let switch_workspace_tool = RegisterToolMessage::new("hyprland_switch_workspace", "Switch to a workspace by ID.", &switch_workspace_schema);
        broadcaster.broadcast_message_to_topic(switch_workspace_tool);

        let move_window_schema = serde_json::to_string(&schema_for!(MoveWindowArgs)).unwrap_or_default();
        let move_window_tool = RegisterToolMessage::new("hyprland_move_window", "Move the active window to a workspace by ID.", &move_window_schema);
        broadcaster.broadcast_message_to_topic(move_window_tool);

        let toggle_floating_schema = serde_json::to_string(&schema_for!(ToggleFloatingArgs)).unwrap_or_default();
        let toggle_floating_tool = RegisterToolMessage::new("hyprland_toggle_floating", "Toggle floating mode for the active window.", &toggle_floating_schema);
        broadcaster.broadcast_message_to_topic(toggle_floating_tool);
    }
}
