use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandWorkspaceIdentifier;

use super::workspace::TOPIC_DISPATCH;

/// Moves the focused window to the specified workspace silently (without switching).
#[derive(Clone, Debug, Default)]
pub struct MoveFocusedWindowToWorkspaceSilentDispatchMessage {
    pub identifier: HyprlandWorkspaceIdentifier,
}

/// ABI-stable version of `MoveFocusedWindowToWorkspaceSilentDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct MoveFocusedWindowToWorkspaceSilentDispatchMessageStabby {
    pub identifier: HyprlandWorkspaceIdentifier,
}

impl From<MoveFocusedWindowToWorkspaceSilentDispatchMessage> for MoveFocusedWindowToWorkspaceSilentDispatchMessageStabby {
    fn from(value: MoveFocusedWindowToWorkspaceSilentDispatchMessage) -> Self {
        Self { identifier: value.identifier }
    }
}

impl From<MoveFocusedWindowToWorkspaceSilentDispatchMessageStabby> for MoveFocusedWindowToWorkspaceSilentDispatchMessage {
    fn from(value: MoveFocusedWindowToWorkspaceSilentDispatchMessageStabby) -> Self {
        Self { identifier: value.identifier }
    }
}

impl TypedMessage for MoveFocusedWindowToWorkspaceSilentDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveFocusedWindowToWorkspaceSilentDispatchMessage");
}

impl TypedMessage for MoveFocusedWindowToWorkspaceSilentDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveFocusedWindowToWorkspaceSilentDispatchMessageStabby");
}

impl MessageTopic for MoveFocusedWindowToWorkspaceSilentDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for MoveFocusedWindowToWorkspaceSilentDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveFocusedWindowToWorkspaceSilentDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
