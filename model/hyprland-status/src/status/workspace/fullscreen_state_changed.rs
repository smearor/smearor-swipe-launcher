use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Emitted when the fullscreen state of the active window changes.
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullscreenStateChangedStatusMessage {
    /// Whether the active window is now fullscreen.
    pub is_fullscreen: bool,
}

impl TypedMessage for FullscreenStateChangedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FullscreenStateChangedStatusMessage");
}
