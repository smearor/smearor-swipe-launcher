use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Data for a window pin event.
/// Mirrors `hyprland::event_listener::WindowPinEventData`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HyprlandWindowPinEventData {
    /// The window address.
    pub window_address: stabby::string::String,
    /// Whether the window is now pinned.
    pub is_pinned: bool,
}

impl TypedMessage for HyprlandWindowPinEventData {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWindowPinEventData");
}
