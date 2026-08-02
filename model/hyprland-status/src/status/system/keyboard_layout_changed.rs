use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::event::layout_event::HyprlandLayoutEvent;

/// Emitted when the keyboard layout changes.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct KeyboardLayoutChangedStatusMessage {
    /// Data about the layout change.
    pub data: HyprlandLayoutEvent,
}

impl TypedMessage for KeyboardLayoutChangedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::KeyboardLayoutChangedStatusMessage");
}
