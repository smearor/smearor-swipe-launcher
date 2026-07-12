use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandWorkspaceIdentifier;

use super::workspace::TOPIC_DISPATCH;

/// Moves the focused window to the specified workspace.
#[derive(Clone, Debug, Default)]
pub struct MoveFocusedWindowToWorkspaceDispatchMessage {
    pub identifier: HyprlandWorkspaceIdentifier,
}

/// ABI-stable version of `MoveFocusedWindowToWorkspaceDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct MoveFocusedWindowToWorkspaceDispatchMessageStabby {
    pub identifier: HyprlandWorkspaceIdentifier,
}

impl From<MoveFocusedWindowToWorkspaceDispatchMessage> for MoveFocusedWindowToWorkspaceDispatchMessageStabby {
    fn from(value: MoveFocusedWindowToWorkspaceDispatchMessage) -> Self {
        Self { identifier: value.identifier }
    }
}

impl From<MoveFocusedWindowToWorkspaceDispatchMessageStabby> for MoveFocusedWindowToWorkspaceDispatchMessage {
    fn from(value: MoveFocusedWindowToWorkspaceDispatchMessageStabby) -> Self {
        Self { identifier: value.identifier }
    }
}

impl TypedMessage for MoveFocusedWindowToWorkspaceDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveFocusedWindowToWorkspaceDispatchMessage");
}

impl TypedMessage for MoveFocusedWindowToWorkspaceDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveFocusedWindowToWorkspaceDispatchMessageStabby");
}

impl MessageTopic for MoveFocusedWindowToWorkspaceDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for MoveFocusedWindowToWorkspaceDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveFocusedWindowToWorkspaceDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
