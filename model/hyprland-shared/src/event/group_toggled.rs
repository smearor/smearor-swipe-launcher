use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Data for a group toggled event.
/// Mirrors `hyprland::event_listener::GroupToggledEventData`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HyprlandGroupToggledEventData {
    /// The window address that was toggled.
    pub window_address: stabby::string::String,
    /// Whether the window is now in a group.
    pub is_grouped: bool,
}

impl TypedMessage for HyprlandGroupToggledEventData {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandGroupToggledEventData");
}
