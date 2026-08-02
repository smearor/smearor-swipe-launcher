use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::event::group_toggled::HyprlandGroupToggledEventData;

/// Emitted when a window group is toggled.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GroupToggledStatusMessage {
    /// Data about the group toggle.
    pub data: HyprlandGroupToggledEventData,
}

impl TypedMessage for GroupToggledStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::GroupToggledStatusMessage");
}
