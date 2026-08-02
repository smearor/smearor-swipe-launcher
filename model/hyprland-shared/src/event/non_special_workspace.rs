use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Data for a non-special workspace event (workspace renamed, etc.).
/// Mirrors `hyprland::event_listener::NonSpecialWorkspaceEventData`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HyprlandNonSpecialWorkspaceData {
    /// The workspace name.
    pub workspace_name: stabby::string::String,
    /// The workspace ID.
    pub workspace_id: i32,
}

impl TypedMessage for HyprlandNonSpecialWorkspaceData {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandNonSpecialWorkspaceData");
}
