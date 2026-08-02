use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::status::window::ActiveWindowChangedStatusMessage;
use crate::status::window::FloatStateChangedStatusMessage;
use crate::status::window::UrgentStateChangedStatusMessage;
use crate::status::window::WindowClosedStatusMessage;
use crate::status::window::WindowMovedStatusMessage;
use crate::status::window::WindowOpenedStatusMessage;
use crate::status::window::WindowPinnedStatusMessage;
use crate::status::window::WindowTitleChangedStatusMessage;

/// Window-related status events.
#[repr(stabby)]
#[stabby::stabby]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WindowEvent {
    /// The active window changed.
    ActiveChanged(ActiveWindowChangedStatusMessage),
    /// A window was opened.
    Opened(WindowOpenedStatusMessage),
    /// A window was closed.
    Closed(WindowClosedStatusMessage),
    /// A window was moved to a different workspace.
    Moved(WindowMovedStatusMessage),
    /// A window's float state changed.
    FloatStateChanged(FloatStateChangedStatusMessage),
    /// A window's urgent state changed.
    UrgentStateChanged(UrgentStateChangedStatusMessage),
    /// A window's title changed.
    TitleChanged(WindowTitleChangedStatusMessage),
    /// A window was pinned or unpinned.
    Pinned(WindowPinnedStatusMessage),
}

impl TypedMessage for WindowEvent {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WindowEvent");
}
