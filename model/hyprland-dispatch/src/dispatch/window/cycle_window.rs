use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandCycleDirection;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Cycles focus to the next or previous window.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CycleWindowDispatchMessage {
    pub cycle_direction: HyprlandCycleDirection,
}

impl TypedMessage for CycleWindowDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::CycleWindowDispatchMessage");
}

impl MessageTopic for CycleWindowDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for CycleWindowDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
