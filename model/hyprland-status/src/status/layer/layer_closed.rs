use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Emitted when a layer shell surface is closed.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LayerClosedStatusMessage {
    /// The layer namespace name.
    pub layer_name: stabby::string::String,
}

impl TypedMessage for LayerClosedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::LayerClosedStatusMessage");
}
