use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandWorkspaceOptions;

use super::workspace::TOPIC_DISPATCH;

/// Toggles workspace options (all pseudo or all float).
#[derive(Clone, Debug, Default)]
pub struct WorkspaceOptionDispatchMessage {
    pub option: HyprlandWorkspaceOptions,
}

/// ABI-stable version of `WorkspaceOptionDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct WorkspaceOptionDispatchMessageStabby {
    pub option: HyprlandWorkspaceOptions,
}

impl From<WorkspaceOptionDispatchMessage> for WorkspaceOptionDispatchMessageStabby {
    fn from(value: WorkspaceOptionDispatchMessage) -> Self {
        Self { option: value.option }
    }
}

impl From<WorkspaceOptionDispatchMessageStabby> for WorkspaceOptionDispatchMessage {
    fn from(value: WorkspaceOptionDispatchMessageStabby) -> Self {
        Self { option: value.option }
    }
}

impl TypedMessage for WorkspaceOptionDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WorkspaceOptionDispatchMessage");
}

impl TypedMessage for WorkspaceOptionDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WorkspaceOptionDispatchMessageStabby");
}

impl MessageTopic for WorkspaceOptionDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for WorkspaceOptionDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for WorkspaceOptionDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
