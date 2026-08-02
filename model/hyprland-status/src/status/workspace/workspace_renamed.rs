use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::event::non_special_workspace::HyprlandNonSpecialWorkspaceData;

/// Emitted when a workspace is renamed.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceRenamedStatusMessage {
    /// Data about the renamed workspace.
    pub data: HyprlandNonSpecialWorkspaceData,
}

impl TypedMessage for WorkspaceRenamedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WorkspaceRenamedStatusMessage");
}
