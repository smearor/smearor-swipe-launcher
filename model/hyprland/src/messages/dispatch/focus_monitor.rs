use crate::HyprlandMonitorIdentifier;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Focuses the specified monitor.
#[derive(Clone, Debug, Default)]
pub struct FocusMonitorDispatchMessage {
    pub monitor_identifier: HyprlandMonitorIdentifier,
}

/// ABI-stable version of `FocusMonitorDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct FocusMonitorDispatchMessageStabby {
    pub monitor_identifier: HyprlandMonitorIdentifier,
}

impl From<FocusMonitorDispatchMessage> for FocusMonitorDispatchMessageStabby {
    fn from(value: FocusMonitorDispatchMessage) -> Self {
        Self {
            monitor_identifier: value.monitor_identifier,
        }
    }
}

impl From<FocusMonitorDispatchMessageStabby> for FocusMonitorDispatchMessage {
    fn from(value: FocusMonitorDispatchMessageStabby) -> Self {
        Self {
            monitor_identifier: value.monitor_identifier,
        }
    }
}

impl TypedMessage for FocusMonitorDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FocusMonitorDispatchMessage");
}

impl TypedMessage for FocusMonitorDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FocusMonitorDispatchMessageStabby");
}

impl MessageTopic for FocusMonitorDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for FocusMonitorDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for FocusMonitorDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
