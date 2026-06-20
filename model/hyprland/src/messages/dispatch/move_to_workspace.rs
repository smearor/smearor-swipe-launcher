use crate::HyprlandWorkspaceIdentifierWithSpecial;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Moves the active window to the specified workspace.
#[derive(Clone, Debug, Default)]
pub struct MoveToWorkspaceDispatchMessage {
    pub identifier: HyprlandWorkspaceIdentifierWithSpecial,
}

/// ABI-stable version of `MoveToWorkspaceDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct MoveToWorkspaceDispatchMessageStabby {
    pub identifier: HyprlandWorkspaceIdentifierWithSpecial,
}

impl From<MoveToWorkspaceDispatchMessage> for MoveToWorkspaceDispatchMessageStabby {
    fn from(value: MoveToWorkspaceDispatchMessage) -> Self {
        Self { identifier: value.identifier }
    }
}

impl From<MoveToWorkspaceDispatchMessageStabby> for MoveToWorkspaceDispatchMessage {
    fn from(value: MoveToWorkspaceDispatchMessageStabby) -> Self {
        Self { identifier: value.identifier }
    }
}

impl TypedMessage for MoveToWorkspaceDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveToWorkspaceDispatchMessage");
}

impl TypedMessage for MoveToWorkspaceDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveToWorkspaceDispatchMessageStabby");
}

impl MessageTopic for MoveToWorkspaceDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for MoveToWorkspaceDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveToWorkspaceDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
