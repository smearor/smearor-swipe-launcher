use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandMonitorIdentifier;
use crate::HyprlandWorkspaceIdentifier;

use super::workspace::TOPIC_DISPATCH;

/// Moves a workspace to the specified monitor.
#[derive(Clone, Debug, Default)]
pub struct MoveWorkspaceToMonitorDispatchMessage {
    pub workspace_identifier: HyprlandWorkspaceIdentifier,
    pub monitor_identifier: HyprlandMonitorIdentifier,
}

/// ABI-stable version of `MoveWorkspaceToMonitorDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct MoveWorkspaceToMonitorDispatchMessageStabby {
    pub workspace_identifier: HyprlandWorkspaceIdentifier,
    pub monitor_identifier: HyprlandMonitorIdentifier,
}

impl From<MoveWorkspaceToMonitorDispatchMessage> for MoveWorkspaceToMonitorDispatchMessageStabby {
    fn from(value: MoveWorkspaceToMonitorDispatchMessage) -> Self {
        Self {
            workspace_identifier: value.workspace_identifier,
            monitor_identifier: value.monitor_identifier,
        }
    }
}

impl From<MoveWorkspaceToMonitorDispatchMessageStabby> for MoveWorkspaceToMonitorDispatchMessage {
    fn from(value: MoveWorkspaceToMonitorDispatchMessageStabby) -> Self {
        Self {
            workspace_identifier: value.workspace_identifier,
            monitor_identifier: value.monitor_identifier,
        }
    }
}

impl TypedMessage for MoveWorkspaceToMonitorDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveWorkspaceToMonitorDispatchMessage");
}

impl TypedMessage for MoveWorkspaceToMonitorDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveWorkspaceToMonitorDispatchMessageStabby");
}

impl MessageTopic for MoveWorkspaceToMonitorDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for MoveWorkspaceToMonitorDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveWorkspaceToMonitorDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
