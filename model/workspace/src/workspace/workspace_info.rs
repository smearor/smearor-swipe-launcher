use serde::Deserialize;
use serde::Serialize;

/// Information about a single workspace.
///
/// Used in snapshots and internal widget state to represent one workspace
/// in the compositor's workspace list.
#[stabby::stabby]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    /// The workspace ID (numeric, as reported by the compositor).
    /// Set to `-1` for special workspaces.
    pub workspace_id: i32,
    /// The workspace name or number as a string.
    pub workspace_name: stabby::string::String,
    /// The monitor index (0-based, matching GDK display order) on which the
    /// workspace is located.
    pub monitor_index: u32,
    /// Whether this workspace is currently active.
    pub is_active: bool,
}
