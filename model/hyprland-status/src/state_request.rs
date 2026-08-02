use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::event::window_event_data::HyprlandWindowEventData;

/// Topic for Hyprland state requests (Widget -> Service).
pub const TOPIC_HYPRLAND_STATE_REQUEST: &str = "service.hyprland.status.request";

/// Topic for Hyprland state responses (Service -> Widget).
pub const TOPIC_HYPRLAND_STATE: &str = "service.hyprland.status.response";

/// Request the current Hyprland-specific state from the service.
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HyprlandStateRequestMessage {}

impl TypedMessage for HyprlandStateRequestMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandStateRequestMessage");
}

impl MessageTopic for HyprlandStateRequestMessage {
    fn topic() -> &'static str {
        TOPIC_HYPRLAND_STATE_REQUEST
    }
}

/// Current Hyprland-specific state, broadcast by the service in response to a state request.
/// Contains all Hyprland-specific state that a widget needs on startup or reload.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HyprlandStateMessage {
    /// The currently active window, if any.
    pub active_window: stabby::option::Option<HyprlandWindowEventData>,
    /// Whether the active window is fullscreen.
    pub is_fullscreen: bool,
    /// Currently active keyboard layout name, if queryable.
    pub keyboard_layout: stabby::option::Option<stabby::string::String>,
    /// Currently active submap name, if any (empty string = default submap).
    pub sub_map: stabby::string::String,
    /// Whether ignore-group-lock is currently enabled.
    pub ignore_group_lock: bool,
    /// Whether groups are currently locked.
    pub groups_locked: bool,
}

impl TypedMessage for HyprlandStateMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandStateMessage");
}

impl MessageTopic for HyprlandStateMessage {
    fn topic() -> &'static str {
        TOPIC_HYPRLAND_STATE
    }
}
