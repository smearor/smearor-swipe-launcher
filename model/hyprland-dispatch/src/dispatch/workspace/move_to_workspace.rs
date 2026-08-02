use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandWorkspaceIdentifierWithSpecial;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Moves the active window to the specified workspace.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MoveToWorkspaceDispatchMessage {
    pub identifier: HyprlandWorkspaceIdentifierWithSpecial,
}

impl TypedMessage for MoveToWorkspaceDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveToWorkspaceDispatchMessage");
}

impl MessageTopic for MoveToWorkspaceDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveToWorkspaceDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
