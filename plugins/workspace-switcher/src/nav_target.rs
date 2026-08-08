use smearor_model_compositor::CreateWorkspaceMessage;
use smearor_model_compositor::SwitchWorkspaceMessage;
use smearor_model_compositor::WorkspaceCreatePosition;
use smearor_model_compositor::WorkspaceInfo;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;

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
    /// Position for workspace creation when no target exists (`After` for next, `Before` for prev).
    pub create_position: WorkspaceCreatePosition,
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
            create_position: WorkspaceCreatePosition::After,
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
            create_position: WorkspaceCreatePosition::Before,
        })
    }

    /// Broadcasts either a `SwitchWorkspaceMessage` (if a target exists) or a `CreateWorkspaceMessage`
    /// (if no target exists, using the stored create position relative to the anchor).
    pub fn broadcast_switch_or_create(&self, broadcaster: &MessageBroadcasterInner) {
        if self.has_target {
            let msg = SwitchWorkspaceMessage { workspace_id: self.target_id };
            broadcaster.broadcast_message_to_topic(msg);
        } else {
            let msg = CreateWorkspaceMessage {
                relative_to: self.anchor_id,
                position: self.create_position,
            };
            broadcaster.broadcast_message_to_topic(msg);
        }
    }
}
