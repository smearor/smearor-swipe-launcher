use crate::messages::dispatch::AddMasterDispatchMessageStabby;
use crate::messages::dispatch::BringActiveToTopDispatchMessageStabby;
use crate::messages::dispatch::CenterWindowDispatchMessageStabby;
use crate::messages::dispatch::ChangeGroupActiveDispatchMessageStabby;
use crate::messages::dispatch::ChangeSplitRatioDispatchMessageStabby;
use crate::messages::dispatch::CloseWindowDispatchMessageStabby;
use crate::messages::dispatch::CustomDispatchMessageStabby;
use crate::messages::dispatch::CycleWindowDispatchMessageStabby;
use crate::messages::dispatch::ExecDispatchMessageStabby;
use crate::messages::dispatch::ExitDispatchMessageStabby;
use crate::messages::dispatch::FocusCurrentOrLastDispatchMessageStabby;
use crate::messages::dispatch::FocusMasterDispatchMessageStabby;
use crate::messages::dispatch::FocusMonitorDispatchMessageStabby;
use crate::messages::dispatch::FocusUrgentOrLastDispatchMessageStabby;
use crate::messages::dispatch::FocusWindowDispatchMessageStabby;
use crate::messages::dispatch::ForceRendererReloadDispatchMessageStabby;
use crate::messages::dispatch::GlobalDispatchMessageStabby;
use crate::messages::dispatch::KillActiveWindowDispatchMessageStabby;
use crate::messages::dispatch::LockGroupsDispatchMessageStabby;
use crate::messages::dispatch::MoveActiveDispatchMessageStabby;
use crate::messages::dispatch::MoveCurrentWorkspaceToMonitorDispatchMessageStabby;
use crate::messages::dispatch::MoveCursorDispatchMessageStabby;
use crate::messages::dispatch::MoveCursorToCornerDispatchMessageStabby;
use crate::messages::dispatch::MoveFocusDispatchMessageStabby;
use crate::messages::dispatch::MoveFocusedWindowToWorkspaceDispatchMessageStabby;
use crate::messages::dispatch::MoveFocusedWindowToWorkspaceSilentDispatchMessageStabby;
use crate::messages::dispatch::MoveIntoGroupDispatchMessageStabby;
use crate::messages::dispatch::MoveOutOfGroupDispatchMessageStabby;
use crate::messages::dispatch::MoveToWorkspaceDispatchMessageStabby;
use crate::messages::dispatch::MoveToWorkspaceSilentDispatchMessageStabby;
use crate::messages::dispatch::MoveWindowDispatchMessageStabby;
use crate::messages::dispatch::MoveWindowPixelDispatchMessageStabby;
use crate::messages::dispatch::OrientationBottomDispatchMessageStabby;
use crate::messages::dispatch::OrientationCenterDispatchMessageStabby;
use crate::messages::dispatch::OrientationLeftDispatchMessageStabby;
use crate::messages::dispatch::OrientationNextDispatchMessageStabby;
use crate::messages::dispatch::OrientationPrevDispatchMessageStabby;
use crate::messages::dispatch::OrientationRightDispatchMessageStabby;
use crate::messages::dispatch::OrientationTopDispatchMessageStabby;
use crate::messages::dispatch::PassDispatchMessageStabby;
use crate::messages::dispatch::RemoveMasterDispatchMessageStabby;
use crate::messages::dispatch::RenameWorkspaceDispatchMessageStabby;
use crate::messages::dispatch::ResizeActiveDispatchMessageStabby;
use crate::messages::dispatch::ResizeWindowPixelDispatchMessageStabby;
use crate::messages::dispatch::SetCursorDispatchMessageStabby;
use crate::messages::dispatch::SwapActiveWorkspacesDispatchMessageStabby;
use crate::messages::dispatch::SwapWindowDispatchMessageStabby;
use crate::messages::dispatch::SwapWithMasterDispatchMessageStabby;
use crate::messages::dispatch::ToggleDpmsDispatchMessageStabby;
use crate::messages::dispatch::ToggleFakeFullscreenDispatchMessageStabby;
use crate::messages::dispatch::ToggleFloatingDispatchMessageStabby;
use crate::messages::dispatch::ToggleFullscreenDispatchMessageStabby;
use crate::messages::dispatch::ToggleGroupDispatchMessageStabby;
use crate::messages::dispatch::ToggleOpaqueDispatchMessageStabby;
use crate::messages::dispatch::TogglePinDispatchMessageStabby;
use crate::messages::dispatch::TogglePseudoDispatchMessageStabby;
use crate::messages::dispatch::ToggleSpecialWorkspaceDispatchMessageStabby;
use crate::messages::dispatch::ToggleSplitDispatchMessageStabby;
use crate::messages::dispatch::WorkspaceDispatchMessageStabby;
use crate::messages::dispatch::WorkspaceOptionDispatchMessageStabby;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::dispatch::TOPIC_DISPATCH;

/// The kind of Hyprland dispatch command.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandDispatchActionKind {
    #[default]
    AddMaster,
    BringActiveToTop,
    CenterWindow,
    ChangeGroupActive,
    ChangeSplitRatio,
    CloseWindow,
    Custom,
    CycleWindow,
    Exec,
    Exit,
    FocusCurrentOrLast,
    FocusMaster,
    FocusMonitor,
    FocusUrgentOrLast,
    FocusWindow,
    ForceRendererReload,
    Global,
    KillActiveWindow,
    LockGroups,
    MoveActive,
    MoveCursor,
    MoveCursorToCorner,
    MoveCurrentWorkspaceToMonitor,
    MoveFocusedWindowToWorkspace,
    MoveFocusedWindowToWorkspaceSilent,
    MoveFocus,
    MoveIntoGroup,
    MoveOutOfGroup,
    MoveToWorkspace,
    MoveToWorkspaceSilent,
    MoveWindow,
    MoveWindowPixel,
    OrientationBottom,
    OrientationCenter,
    OrientationLeft,
    OrientationNext,
    OrientationPrev,
    OrientationRight,
    OrientationTop,
    Pass,
    RemoveMaster,
    RenameWorkspace,
    ResizeActive,
    ResizeWindowPixel,
    SetCursor,
    SwapActiveWorkspaces,
    SwapWindow,
    SwapWithMaster,
    ToggleDpms,
    ToggleFakeFullscreen,
    ToggleFloating,
    ToggleFullscreen,
    ToggleGroup,
    ToggleOpaque,
    TogglePin,
    TogglePseudo,
    ToggleSpecialWorkspace,
    ToggleSplit,
    Workspace,
    WorkspaceOption,
}

/// The main dispatch envelope sent by widgets.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct HyprlandDispatchMessage {
    pub kind: HyprlandDispatchActionKind,
    pub add_master: stabby::option::Option<AddMasterDispatchMessageStabby>,
    pub bring_active_to_top: stabby::option::Option<BringActiveToTopDispatchMessageStabby>,
    pub center_window: stabby::option::Option<CenterWindowDispatchMessageStabby>,
    pub change_group_active: stabby::option::Option<ChangeGroupActiveDispatchMessageStabby>,
    pub change_split_ratio: stabby::option::Option<ChangeSplitRatioDispatchMessageStabby>,
    pub close_window: stabby::option::Option<CloseWindowDispatchMessageStabby>,
    pub custom: stabby::option::Option<CustomDispatchMessageStabby>,
    pub cycle_window: stabby::option::Option<CycleWindowDispatchMessageStabby>,
    pub exec: stabby::option::Option<ExecDispatchMessageStabby>,
    pub exit: stabby::option::Option<ExitDispatchMessageStabby>,
    pub focus_current_or_last: stabby::option::Option<FocusCurrentOrLastDispatchMessageStabby>,
    pub focus_master: stabby::option::Option<FocusMasterDispatchMessageStabby>,
    pub focus_monitor: stabby::option::Option<FocusMonitorDispatchMessageStabby>,
    pub focus_urgent_or_last: stabby::option::Option<FocusUrgentOrLastDispatchMessageStabby>,
    pub focus_window: stabby::option::Option<FocusWindowDispatchMessageStabby>,
    pub force_renderer_reload: stabby::option::Option<ForceRendererReloadDispatchMessageStabby>,
    pub global: stabby::option::Option<GlobalDispatchMessageStabby>,
    pub kill_active_window: stabby::option::Option<KillActiveWindowDispatchMessageStabby>,
    pub lock_groups: stabby::option::Option<LockGroupsDispatchMessageStabby>,
    pub move_active: stabby::option::Option<MoveActiveDispatchMessageStabby>,
    pub move_cursor: stabby::option::Option<MoveCursorDispatchMessageStabby>,
    pub move_cursor_to_corner: stabby::option::Option<MoveCursorToCornerDispatchMessageStabby>,
    pub move_current_workspace_to_monitor: stabby::option::Option<MoveCurrentWorkspaceToMonitorDispatchMessageStabby>,
    pub move_focused_window_to_workspace: stabby::option::Option<MoveFocusedWindowToWorkspaceDispatchMessageStabby>,
    pub move_focused_window_to_workspace_silent: stabby::option::Option<MoveFocusedWindowToWorkspaceSilentDispatchMessageStabby>,
    pub move_focus: stabby::option::Option<MoveFocusDispatchMessageStabby>,
    pub move_into_group: stabby::option::Option<MoveIntoGroupDispatchMessageStabby>,
    pub move_out_of_group: stabby::option::Option<MoveOutOfGroupDispatchMessageStabby>,
    pub move_to_workspace: stabby::option::Option<MoveToWorkspaceDispatchMessageStabby>,
    pub move_to_workspace_silent: stabby::option::Option<MoveToWorkspaceSilentDispatchMessageStabby>,
    pub move_window: stabby::option::Option<MoveWindowDispatchMessageStabby>,
    pub move_window_pixel: stabby::option::Option<MoveWindowPixelDispatchMessageStabby>,
    pub orientation_bottom: stabby::option::Option<OrientationBottomDispatchMessageStabby>,
    pub orientation_center: stabby::option::Option<OrientationCenterDispatchMessageStabby>,
    pub orientation_left: stabby::option::Option<OrientationLeftDispatchMessageStabby>,
    pub orientation_next: stabby::option::Option<OrientationNextDispatchMessageStabby>,
    pub orientation_prev: stabby::option::Option<OrientationPrevDispatchMessageStabby>,
    pub orientation_right: stabby::option::Option<OrientationRightDispatchMessageStabby>,
    pub orientation_top: stabby::option::Option<OrientationTopDispatchMessageStabby>,
    pub pass: stabby::option::Option<PassDispatchMessageStabby>,
    pub remove_master: stabby::option::Option<RemoveMasterDispatchMessageStabby>,
    pub rename_workspace: stabby::option::Option<RenameWorkspaceDispatchMessageStabby>,
    pub resize_active: stabby::option::Option<ResizeActiveDispatchMessageStabby>,
    pub resize_window_pixel: stabby::option::Option<ResizeWindowPixelDispatchMessageStabby>,
    pub set_cursor: stabby::option::Option<SetCursorDispatchMessageStabby>,
    pub swap_active_workspaces: stabby::option::Option<SwapActiveWorkspacesDispatchMessageStabby>,
    pub swap_window: stabby::option::Option<SwapWindowDispatchMessageStabby>,
    pub swap_with_master: stabby::option::Option<SwapWithMasterDispatchMessageStabby>,
    pub toggle_dpms: stabby::option::Option<ToggleDpmsDispatchMessageStabby>,
    pub toggle_fake_fullscreen: stabby::option::Option<ToggleFakeFullscreenDispatchMessageStabby>,
    pub toggle_floating: stabby::option::Option<ToggleFloatingDispatchMessageStabby>,
    pub toggle_fullscreen: stabby::option::Option<ToggleFullscreenDispatchMessageStabby>,
    pub toggle_group: stabby::option::Option<ToggleGroupDispatchMessageStabby>,
    pub toggle_opaque: stabby::option::Option<ToggleOpaqueDispatchMessageStabby>,
    pub toggle_pin: stabby::option::Option<TogglePinDispatchMessageStabby>,
    pub toggle_pseudo: stabby::option::Option<TogglePseudoDispatchMessageStabby>,
    pub toggle_special_workspace: stabby::option::Option<ToggleSpecialWorkspaceDispatchMessageStabby>,
    pub toggle_split: stabby::option::Option<ToggleSplitDispatchMessageStabby>,
    pub workspace: stabby::option::Option<WorkspaceDispatchMessageStabby>,
    pub workspace_option: stabby::option::Option<WorkspaceOptionDispatchMessageStabby>,
}

impl TypedMessage for HyprlandDispatchActionKind {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandDispatchActionKind");
}

impl TypedMessage for HyprlandDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandDispatchMessage");
}

impl MessageTopic for HyprlandDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for HyprlandDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
