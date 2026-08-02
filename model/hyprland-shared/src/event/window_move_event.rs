use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Data for a window move event.
/// Mirrors `hyprland::event_listener::WindowMoveEvent`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HyprlandWindowMoveEvent {
    /// The window address being moved.
    pub window_address: stabby::string::String,
    /// The workspace ID the window is being moved to.
    pub workspace_id: i32,
}

impl TypedMessage for HyprlandWindowMoveEvent {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWindowMoveEvent");
}
