use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Data for a changed special workspace event.
/// Mirrors `hyprland::event_listener::ChangedSpecialEventData`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HyprlandChangedSpecialEventData {
    /// The monitor name.
    pub monitor_name: stabby::string::String,
    /// The special workspace name.
    pub special_workspace_name: stabby::string::String,
}

impl TypedMessage for HyprlandChangedSpecialEventData {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandChangedSpecialEventData");
}
