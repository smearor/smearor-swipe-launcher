use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::event::screencast::HyprlandScreencastEventData;

/// Emitted when a screencast state changes.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ScreencastStatusMessage {
    /// Data about the screencast state change.
    pub data: HyprlandScreencastEventData,
}

impl TypedMessage for ScreencastStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ScreencastStatusMessage");
}
