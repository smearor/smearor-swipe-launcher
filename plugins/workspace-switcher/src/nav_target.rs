use smearor_model_compositor::WorkspaceInfo;

/// Navigation target computed from the current workspace list and view index.
///
/// Used by `next_view`/`prev_view` in the main widget and `next_workspace`/`prev_workspace`
/// in the atomic widget to determine whether to switch to an adjacent workspace or create a new one.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceNavTarget {
    /// Current index in the workspace list.
    pub idx: usize,
    /// Whether an adjacent workspace exists in the navigation direction.
    pub has_target: bool,
    /// Workspace ID to switch to, or -1 if no target exists.
    pub target_id: i32,
    /// Workspace ID of the current workspace, used as anchor for create-relative positioning.
    pub anchor_id: i32,
}

impl WorkspaceNavTarget {
    /// Computes the navigation target for moving to the next workspace.
    pub fn next(workspaces: &[WorkspaceInfo], current_idx: usize) -> Option<Self> {
        if workspaces.is_empty() {
            return None;
        }
        let has_target = current_idx + 1 < workspaces.len();
        let target_id = if has_target { workspaces[current_idx + 1].workspace_id } else { -1 };
        let anchor_id = workspaces[current_idx].workspace_id;
        Some(Self {
            idx: current_idx,
            has_target,
            target_id,
            anchor_id,
        })
    }

    /// Computes the navigation target for moving to the previous workspace.
    pub fn prev(workspaces: &[WorkspaceInfo], current_idx: usize) -> Option<Self> {
        if workspaces.is_empty() {
            return None;
        }
        let has_target = current_idx > 0;
        let target_id = if has_target { workspaces[current_idx - 1].workspace_id } else { -1 };
        let anchor_id = workspaces[current_idx].workspace_id;
        Some(Self {
            idx: current_idx,
            has_target,
            target_id,
            anchor_id,
        })
    }
}
