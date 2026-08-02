use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Emitted when the submap changes.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SubMapChangedStatusMessage {
    /// The new submap name.
    pub sub_map: stabby::string::String,
}

impl TypedMessage for SubMapChangedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SubMapChangedStatusMessage");
}
