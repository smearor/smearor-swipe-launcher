use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `hyprland_switch_workspace` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SwitchWorkspaceArgs {
    /// Workspace ID to switch to
    pub workspace_id: i32,
}

/// Arguments for the `hyprland_move_window` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MoveWindowArgs {
    /// Workspace ID to move the focused window to
    pub workspace_id: i32,
}

/// Arguments for the `hyprland_toggle_floating` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ToggleFloatingArgs {}
