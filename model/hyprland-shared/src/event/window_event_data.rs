use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Data for a window event (active window change, urgent state, etc.).
/// Mirrors `hyprland::event_listener::WindowEventData`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HyprlandWindowEventData {
    /// The window class (e.g. "firefox", "kitty").
    pub window_class: stabby::string::String,
    /// The window title.
    pub window_title: stabby::string::String,
    /// The window address as a string (e.g. "0x1234567").
    pub window_address: stabby::string::String,
    /// The workspace ID the window is on, if available.
    pub workspace_id: stabby::option::Option<i32>,
}

impl TypedMessage for HyprlandWindowEventData {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWindowEventData");
}
