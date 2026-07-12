use crate::HyprlandCycleDirection;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Cycles focus to the next or previous window.
#[derive(Clone, Debug, Default)]
pub struct CycleWindowDispatchMessage {
    pub cycle_direction: HyprlandCycleDirection,
}

/// ABI-stable version of `CycleWindowDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct CycleWindowDispatchMessageStabby {
    pub cycle_direction: HyprlandCycleDirection,
}

impl From<CycleWindowDispatchMessage> for CycleWindowDispatchMessageStabby {
    fn from(value: CycleWindowDispatchMessage) -> Self {
        Self {
            cycle_direction: value.cycle_direction,
        }
    }
}

impl From<CycleWindowDispatchMessageStabby> for CycleWindowDispatchMessage {
    fn from(value: CycleWindowDispatchMessageStabby) -> Self {
        Self {
            cycle_direction: value.cycle_direction,
        }
    }
}

impl TypedMessage for CycleWindowDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::CycleWindowDispatchMessage");
}

impl TypedMessage for CycleWindowDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::CycleWindowDispatchMessageStabby");
}

impl MessageTopic for CycleWindowDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for CycleWindowDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for CycleWindowDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
