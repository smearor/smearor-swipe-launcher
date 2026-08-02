use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::window::CenterWindowDispatchMessage;
use crate::dispatch::window::ChangeGroupActiveDispatchMessage;
use crate::dispatch::window::ChangeSplitRatioDispatchMessage;
use crate::dispatch::window::CloseWindowDispatchMessage;
use crate::dispatch::window::CycleWindowDispatchMessage;
use crate::dispatch::window::ExecDispatchMessageStabby;
use crate::dispatch::window::FocusCurrentOrLastDispatchMessage;
use crate::dispatch::window::FocusMasterDispatchMessage;
use crate::dispatch::window::FocusMonitorDispatchMessage;
use crate::dispatch::window::FocusUrgentOrLastDispatchMessage;
use crate::dispatch::window::FocusWindowDispatchMessage;
use crate::dispatch::window::KillActiveWindowDispatchMessage;
use crate::dispatch::window::MoveActiveDispatchMessage;
use crate::dispatch::window::MoveCursorDispatchMessage;
use crate::dispatch::window::MoveCursorToCornerDispatchMessage;
use crate::dispatch::window::MoveFocusDispatchMessage;
use crate::dispatch::window::MoveIntoGroupDispatchMessage;
use crate::dispatch::window::MoveWindowDispatchMessage;
use crate::dispatch::window::MoveWindowPixelDispatchMessage;
use crate::dispatch::window::ResizeActiveDispatchMessage;
use crate::dispatch::window::ResizeWindowPixelDispatchMessage;
use crate::dispatch::window::SwapWindowDispatchMessage;
use crate::dispatch::window::SwapWithMasterDispatchMessage;

/// Kind for window-related dispatch commands.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowDispatchKind {
    #[default]
    CenterWindow,
    ChangeGroupActive,
    ChangeSplitRatio,
    CloseWindow,
    CycleWindow,
    Exec,
    FocusCurrentOrLast,
    FocusMaster,
    FocusMonitor,
    FocusUrgentOrLast,
    FocusWindow,
    KillActiveWindow,
    MoveActive,
    MoveCursor,
    MoveCursorToCorner,
    MoveFocus,
    MoveIntoGroup,
    MoveWindow,
    MoveWindowPixel,
    ResizeActive,
    ResizeWindowPixel,
    SwapWindow,
    SwapWithMaster,
}

/// Window-related dispatch options (focus, move, resize, close, swap, cycle).
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WindowDispatchOps {
    pub center_window: stabby::option::Option<CenterWindowDispatchMessage>,
    pub change_group_active: stabby::option::Option<ChangeGroupActiveDispatchMessage>,
    pub change_split_ratio: stabby::option::Option<ChangeSplitRatioDispatchMessage>,
    pub close_window: stabby::option::Option<CloseWindowDispatchMessage>,
    pub cycle_window: stabby::option::Option<CycleWindowDispatchMessage>,
    pub exec: stabby::option::Option<ExecDispatchMessageStabby>,
    pub focus_current_or_last: stabby::option::Option<FocusCurrentOrLastDispatchMessage>,
    pub focus_master: stabby::option::Option<FocusMasterDispatchMessage>,
    pub focus_monitor: stabby::option::Option<FocusMonitorDispatchMessage>,
    pub focus_urgent_or_last: stabby::option::Option<FocusUrgentOrLastDispatchMessage>,
    pub focus_window: stabby::option::Option<FocusWindowDispatchMessage>,
    pub kill_active_window: stabby::option::Option<KillActiveWindowDispatchMessage>,
    pub move_active: stabby::option::Option<MoveActiveDispatchMessage>,
    pub move_cursor: stabby::option::Option<MoveCursorDispatchMessage>,
    pub move_cursor_to_corner: stabby::option::Option<MoveCursorToCornerDispatchMessage>,
    pub move_focus: stabby::option::Option<MoveFocusDispatchMessage>,
    pub move_into_group: stabby::option::Option<MoveIntoGroupDispatchMessage>,
    pub move_window: stabby::option::Option<MoveWindowDispatchMessage>,
    pub move_window_pixel: stabby::option::Option<MoveWindowPixelDispatchMessage>,
    pub resize_active: stabby::option::Option<ResizeActiveDispatchMessage>,
    pub resize_window_pixel: stabby::option::Option<ResizeWindowPixelDispatchMessage>,
    pub swap_window: stabby::option::Option<SwapWindowDispatchMessage>,
    pub swap_with_master: stabby::option::Option<SwapWithMasterDispatchMessage>,
}

impl TypedMessage for WindowDispatchKind {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WindowDispatchKind");
}
