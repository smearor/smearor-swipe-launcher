use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandMonitorIdentifier;

use super::workspace::TOPIC_DISPATCH;

/// Swaps the active workspaces between two monitors.
#[derive(Clone, Debug, Default)]
pub struct SwapActiveWorkspacesDispatchMessage {
    pub monitor_a: HyprlandMonitorIdentifier,
    pub monitor_b: HyprlandMonitorIdentifier,
}

/// ABI-stable version of `SwapActiveWorkspacesDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct SwapActiveWorkspacesDispatchMessageStabby {
    pub monitor_a: HyprlandMonitorIdentifier,
    pub monitor_b: HyprlandMonitorIdentifier,
}

impl From<SwapActiveWorkspacesDispatchMessage> for SwapActiveWorkspacesDispatchMessageStabby {
    fn from(value: SwapActiveWorkspacesDispatchMessage) -> Self {
        Self {
            monitor_a: value.monitor_a,
            monitor_b: value.monitor_b,
        }
    }
}

impl From<SwapActiveWorkspacesDispatchMessageStabby> for SwapActiveWorkspacesDispatchMessage {
    fn from(value: SwapActiveWorkspacesDispatchMessageStabby) -> Self {
        Self {
            monitor_a: value.monitor_a,
            monitor_b: value.monitor_b,
        }
    }
}

impl TypedMessage for SwapActiveWorkspacesDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SwapActiveWorkspacesDispatchMessage");
}

impl TypedMessage for SwapActiveWorkspacesDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SwapActiveWorkspacesDispatchMessageStabby");
}

impl MessageTopic for SwapActiveWorkspacesDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for SwapActiveWorkspacesDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for SwapActiveWorkspacesDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
