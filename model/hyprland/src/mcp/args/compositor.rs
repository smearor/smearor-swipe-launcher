use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Position for creating a new workspace relative to a reference workspace.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum WorkspaceCreatePosition {
    /// Create the new workspace before the reference workspace.
    #[default]
    Before,
    /// Create the new workspace after the reference workspace.
    After,
}

/// Arguments for the `hyprland_compositor_create_workspace` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct CreateWorkspaceArgs {
    /// The workspace ID of the reference workspace
    pub relative_to: i32,
    /// Whether to create the new workspace before or after the reference
    pub position: WorkspaceCreatePosition,
}

/// Arguments for the `hyprland_compositor_switch_workspace` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SwitchWorkspaceCompositorArgs {
    /// The workspace ID to switch to
    pub workspace_id: i32,
}
