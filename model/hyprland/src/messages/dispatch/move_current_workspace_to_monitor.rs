use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandMonitorIdentifier;

use super::workspace::TOPIC_DISPATCH;

/// Moves the current workspace to the specified monitor.
#[derive(Clone, Debug, Default)]
pub struct MoveCurrentWorkspaceToMonitorDispatchMessage {
    pub monitor_identifier: HyprlandMonitorIdentifier,
}

/// ABI-stable version of `MoveCurrentWorkspaceToMonitorDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct MoveCurrentWorkspaceToMonitorDispatchMessageStabby {
    pub monitor_identifier: HyprlandMonitorIdentifier,
}

impl From<MoveCurrentWorkspaceToMonitorDispatchMessage> for MoveCurrentWorkspaceToMonitorDispatchMessageStabby {
    fn from(value: MoveCurrentWorkspaceToMonitorDispatchMessage) -> Self {
        Self {
            monitor_identifier: value.monitor_identifier,
        }
    }
}

impl From<MoveCurrentWorkspaceToMonitorDispatchMessageStabby> for MoveCurrentWorkspaceToMonitorDispatchMessage {
    fn from(value: MoveCurrentWorkspaceToMonitorDispatchMessageStabby) -> Self {
        Self {
            monitor_identifier: value.monitor_identifier,
        }
    }
}

impl TypedMessage for MoveCurrentWorkspaceToMonitorDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveCurrentWorkspaceToMonitorDispatchMessage");
}

impl TypedMessage for MoveCurrentWorkspaceToMonitorDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveCurrentWorkspaceToMonitorDispatchMessageStabby");
}

impl MessageTopic for MoveCurrentWorkspaceToMonitorDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for MoveCurrentWorkspaceToMonitorDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveCurrentWorkspaceToMonitorDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
