use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Emitted when a special workspace is removed.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpecialRemovedStatusMessage {
    /// The monitor name from which the special workspace was removed.
    pub monitor_name: stabby::string::String,
}

impl TypedMessage for SpecialRemovedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SpecialRemovedStatusMessage");
}
