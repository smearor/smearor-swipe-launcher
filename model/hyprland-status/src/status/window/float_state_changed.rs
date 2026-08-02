use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::event::window_float_event_data::HyprlandWindowFloatEventData;

/// Emitted when a window's float state changes.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FloatStateChangedStatusMessage {
    /// Data about the float state change.
    pub data: HyprlandWindowFloatEventData,
}

impl TypedMessage for FloatStateChangedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FloatStateChangedStatusMessage");
}
