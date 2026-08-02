use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::event::window_event_data::HyprlandWindowEventData;

/// Data for a window open event.
/// Mirrors `hyprland::event_listener::WindowOpenEvent`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HyprlandWindowOpenEvent {
    /// The window data.
    pub data: HyprlandWindowEventData,
    /// Whether the window was opened in floating mode.
    pub floats: bool,
    /// The workspace name the window was opened on.
    pub workspace_name: stabby::string::String,
}

impl TypedMessage for HyprlandWindowOpenEvent {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWindowOpenEvent");
}
