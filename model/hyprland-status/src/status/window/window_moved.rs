use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::event::window_move_event::HyprlandWindowMoveEvent;

/// Emitted when a window is moved to a different workspace.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WindowMovedStatusMessage {
    /// Data about the move.
    pub data: HyprlandWindowMoveEvent,
}

impl TypedMessage for WindowMovedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WindowMovedStatusMessage");
}
