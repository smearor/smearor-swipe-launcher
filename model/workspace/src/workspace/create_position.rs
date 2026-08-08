use serde::Deserialize;
use serde::Serialize;

/// Position for creating a new workspace relative to a reference workspace.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceCreatePosition {
    /// Create the new workspace before the reference workspace.
    #[default]
    Before,
    /// Create the new workspace after the reference workspace.
    After,
}
