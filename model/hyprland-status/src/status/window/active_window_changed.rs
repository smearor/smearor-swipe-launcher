use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::event::window_event_data::HyprlandWindowEventData;

/// Emitted when the active window changes.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ActiveWindowChangedStatusMessage {
    /// The newly active window, or `None` if no window is focused.
    pub data: stabby::option::Option<HyprlandWindowEventData>,
}

impl TypedMessage for ActiveWindowChangedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ActiveWindowChangedStatusMessage");
}
