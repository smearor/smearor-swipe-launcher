use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Emitted when the Hyprland config is reloaded.
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigReloadedStatusMessage {}

impl TypedMessage for ConfigReloadedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ConfigReloadedStatusMessage");
}
