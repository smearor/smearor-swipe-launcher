use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::HyprlandMonitorIdentifier;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Moves the current workspace to the specified monitor.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MoveCurrentWorkspaceToMonitorDispatchMessage {
    pub monitor_identifier: HyprlandMonitorIdentifier,
}

impl TypedMessage for MoveCurrentWorkspaceToMonitorDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveCurrentWorkspaceToMonitorDispatchMessage");
}

impl MessageTopic for MoveCurrentWorkspaceToMonitorDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveCurrentWorkspaceToMonitorDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
