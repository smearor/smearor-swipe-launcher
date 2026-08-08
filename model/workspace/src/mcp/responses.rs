use serde::Deserialize;
use serde::Serialize;

/// Single workspace entry in the MCP workspaces resource response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    /// Workspace ID
    pub id: i32,
    /// Workspace name
    pub name: String,
    /// Monitor index the workspace is on
    pub monitor_index: u32,
    /// Whether this workspace is currently active
    pub is_active: bool,
}

/// Response body for the `compositor://workspaces` MCP resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspacesResponse {
    /// Active workspace ID
    pub active_workspace_id: i32,
    /// Active monitor index
    pub active_monitor_index: u32,
    /// List of all workspaces
    pub workspaces: Vec<WorkspaceEntry>,
}
