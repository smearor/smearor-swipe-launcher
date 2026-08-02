use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::HyprlandMonitorIdentifier;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Swaps the active workspaces between two monitors.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SwapActiveWorkspacesDispatchMessage {
    pub monitor_a: HyprlandMonitorIdentifier,
    pub monitor_b: HyprlandMonitorIdentifier,
}

impl TypedMessage for SwapActiveWorkspacesDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SwapActiveWorkspacesDispatchMessage");
}

impl MessageTopic for SwapActiveWorkspacesDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for SwapActiveWorkspacesDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
