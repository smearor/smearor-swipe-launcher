use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Data for a keyboard layout change event.
/// Mirrors `hyprland::event_listener::LayoutEvent`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HyprlandLayoutEvent {
    /// The keyboard name (e.g. "keyboard-0").
    pub keyboard_name: stabby::string::String,
    /// The active layout name.
    pub layout_name: stabby::string::String,
}

impl TypedMessage for HyprlandLayoutEvent {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandLayoutEvent");
}
