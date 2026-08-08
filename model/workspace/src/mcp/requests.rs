use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `compositor_switch_workspace` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SwitchWorkspaceArgs {
    /// Workspace ID to switch to
    pub workspace_id: i32,
}
