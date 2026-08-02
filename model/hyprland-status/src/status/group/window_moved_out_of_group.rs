use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Emitted when a window is moved out of a group.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WindowMovedOutOfGroupStatusMessage {
    /// The address of the window moved out of the group.
    pub window_address: stabby::string::String,
}

impl TypedMessage for WindowMovedOutOfGroupStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WindowMovedOutOfGroupStatusMessage");
}
