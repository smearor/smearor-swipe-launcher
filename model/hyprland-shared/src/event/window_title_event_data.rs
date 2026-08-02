use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Data for a window title change event.
/// Mirrors `hyprland::event_listener::WindowTitleEventData`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HyprlandWindowTitleEventData {
    /// The window address.
    pub window_address: stabby::string::String,
    /// The new window title.
    pub window_title: stabby::string::String,
}

impl TypedMessage for HyprlandWindowTitleEventData {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWindowTitleEventData");
}
