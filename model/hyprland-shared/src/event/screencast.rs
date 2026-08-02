use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Data for a screencast event.
/// Mirrors `hyprland::event_listener::ScreencastEventData`.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HyprlandScreencastType {
    #[default]
    /// A monitor is being screencast.
    Monitor,
    /// A window is being screencast.
    Window,
}

/// Data for a screencast state change event.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HyprlandScreencastEventData {
    /// The type of screencast (monitor or window).
    pub screencast_type: HyprlandScreencastType,
    /// Whether the screencast is now active.
    pub is_active: bool,
    /// The owner (monitor name or window address).
    pub owner: stabby::string::String,
}

impl TypedMessage for HyprlandScreencastType {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandScreencastType");
}

impl TypedMessage for HyprlandScreencastEventData {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandScreencastEventData");
}
