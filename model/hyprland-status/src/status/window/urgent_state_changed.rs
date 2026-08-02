use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Emitted when a window's urgent state changes.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UrgentStateChangedStatusMessage {
    /// The address of the window whose urgent state changed.
    pub window_address: stabby::string::String,
}

impl TypedMessage for UrgentStateChangedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::UrgentStateChangedStatusMessage");
}
