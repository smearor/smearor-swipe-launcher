use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::event::window_open_event::HyprlandWindowOpenEvent;

/// Emitted when a new window is opened.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WindowOpenedStatusMessage {
    /// Data about the opened window.
    pub data: HyprlandWindowOpenEvent,
}

impl TypedMessage for WindowOpenedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WindowOpenedStatusMessage");
}
