use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::MoveCurrentWorkspaceToMonitorDispatchMessage;
use crate::dispatch::workspace::MoveFocusedWindowToWorkspaceDispatchMessage;
use crate::dispatch::workspace::MoveFocusedWindowToWorkspaceSilentDispatchMessageStabby;
use crate::dispatch::workspace::MoveToWorkspaceDispatchMessage;
use crate::dispatch::workspace::MoveToWorkspaceSilentDispatchMessageStabby;
use crate::dispatch::workspace::RenameWorkspaceDispatchMessageStabby;
use crate::dispatch::workspace::SwapActiveWorkspacesDispatchMessage;
use crate::dispatch::workspace::ToggleSpecialWorkspaceDispatchMessageStabby;
use crate::dispatch::workspace::WorkspaceDispatchMessage;
use crate::dispatch::workspace::WorkspaceOptionDispatchMessage;

/// Kind for workspace-related dispatch commands.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceDispatchKind {
    #[default]
    MoveCurrentWorkspaceToMonitor,
    MoveFocusedWindowToWorkspace,
    MoveFocusedWindowToWorkspaceSilent,
    MoveToWorkspace,
    MoveToWorkspaceSilent,
    RenameWorkspace,
    SwapActiveWorkspaces,
    ToggleSpecialWorkspace,
    Workspace,
    WorkspaceOption,
}

/// Workspace-related dispatch options.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceDispatchOps {
    pub move_current_workspace_to_monitor: stabby::option::Option<MoveCurrentWorkspaceToMonitorDispatchMessage>,
    pub move_focused_window_to_workspace: stabby::option::Option<MoveFocusedWindowToWorkspaceDispatchMessage>,
    pub move_focused_window_to_workspace_silent: stabby::option::Option<MoveFocusedWindowToWorkspaceSilentDispatchMessageStabby>,
    pub move_to_workspace: stabby::option::Option<MoveToWorkspaceDispatchMessage>,
    pub move_to_workspace_silent: stabby::option::Option<MoveToWorkspaceSilentDispatchMessageStabby>,
    pub rename_workspace: stabby::option::Option<RenameWorkspaceDispatchMessageStabby>,
    pub swap_active_workspaces: stabby::option::Option<SwapActiveWorkspacesDispatchMessage>,
    pub toggle_special_workspace: stabby::option::Option<ToggleSpecialWorkspaceDispatchMessageStabby>,
    pub workspace: stabby::option::Option<WorkspaceDispatchMessage>,
    pub workspace_option: stabby::option::Option<WorkspaceOptionDispatchMessage>,
}

impl TypedMessage for WorkspaceDispatchKind {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WorkspaceDispatchKind");
}
