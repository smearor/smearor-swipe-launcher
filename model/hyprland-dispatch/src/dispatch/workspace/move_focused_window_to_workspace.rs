use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::HyprlandWorkspaceIdentifier;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Moves the focused window to the specified workspace.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MoveFocusedWindowToWorkspaceDispatchMessage {
    pub identifier: HyprlandWorkspaceIdentifier,
}

impl TypedMessage for MoveFocusedWindowToWorkspaceDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveFocusedWindowToWorkspaceDispatchMessage");
}

impl MessageTopic for MoveFocusedWindowToWorkspaceDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveFocusedWindowToWorkspaceDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
