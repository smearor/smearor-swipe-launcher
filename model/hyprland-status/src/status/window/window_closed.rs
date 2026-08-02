use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Emitted when a window is closed.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WindowClosedStatusMessage {
    /// The address of the closed window.
    pub window_address: stabby::string::String,
}

impl TypedMessage for WindowClosedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WindowClosedStatusMessage");
}
