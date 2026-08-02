use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::event::window_title_event_data::HyprlandWindowTitleEventData;

/// Emitted when a window's title changes.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WindowTitleChangedStatusMessage {
    /// Data about the title change.
    pub data: HyprlandWindowTitleEventData,
}

impl TypedMessage for WindowTitleChangedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WindowTitleChangedStatusMessage");
}
