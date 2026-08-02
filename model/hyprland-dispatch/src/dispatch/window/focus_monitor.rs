use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandMonitorIdentifier;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Focuses the specified monitor.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FocusMonitorDispatchMessage {
    pub monitor_identifier: HyprlandMonitorIdentifier,
}

impl TypedMessage for FocusMonitorDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FocusMonitorDispatchMessage");
}

impl MessageTopic for FocusMonitorDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for FocusMonitorDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
