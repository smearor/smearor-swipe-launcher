use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::event::window_pin::HyprlandWindowPinEventData;

/// Emitted when a window is pinned or unpinned.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WindowPinnedStatusMessage {
    /// Data about the pin state change.
    pub data: HyprlandWindowPinEventData,
}

impl TypedMessage for WindowPinnedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WindowPinnedStatusMessage");
}
