use crate::HyprlandWorkspaceIdentifierWithSpecial;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_DISPATCH: &str = "service.hyprland.dispatch";

/// Switches to the specified workspace.
#[derive(Clone, Debug, Default)]
pub struct WorkspaceDispatchMessage {
    pub identifier: HyprlandWorkspaceIdentifierWithSpecial,
}

/// ABI-stable version of `WorkspaceDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct WorkspaceDispatchMessageStabby {
    pub identifier: HyprlandWorkspaceIdentifierWithSpecial,
}

impl From<WorkspaceDispatchMessage> for WorkspaceDispatchMessageStabby {
    fn from(value: WorkspaceDispatchMessage) -> Self {
        Self { identifier: value.identifier }
    }
}

impl From<WorkspaceDispatchMessageStabby> for WorkspaceDispatchMessage {
    fn from(value: WorkspaceDispatchMessageStabby) -> Self {
        Self { identifier: value.identifier }
    }
}

impl TypedMessage for WorkspaceDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WorkspaceDispatchMessage");
}

impl TypedMessage for WorkspaceDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WorkspaceDispatchMessageStabby");
}

impl MessageTopic for WorkspaceDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for WorkspaceDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for WorkspaceDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
