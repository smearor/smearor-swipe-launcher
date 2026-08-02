use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::status::layer::LayerClosedStatusMessage;
use crate::status::layer::LayerOpenedStatusMessage;

/// Layer shell surface status events.
#[repr(stabby)]
#[stabby::stabby]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LayerEvent {
    /// A layer shell surface was opened.
    Opened(LayerOpenedStatusMessage),
    /// A layer shell surface was closed.
    Closed(LayerClosedStatusMessage),
}

impl TypedMessage for LayerEvent {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::LayerEvent");
}
