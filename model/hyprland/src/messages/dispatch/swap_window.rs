use crate::HyprlandCycleDirection;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Swaps the active window with the next or previous in the cycle.
#[derive(Clone, Debug, Default)]
pub struct SwapWindowDispatchMessage {
    pub cycle_direction: HyprlandCycleDirection,
}

/// ABI-stable version of `SwapWindowDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct SwapWindowDispatchMessageStabby {
    pub cycle_direction: HyprlandCycleDirection,
}

impl From<SwapWindowDispatchMessage> for SwapWindowDispatchMessageStabby {
    fn from(value: SwapWindowDispatchMessage) -> Self {
        Self {
            cycle_direction: value.cycle_direction,
        }
    }
}

impl From<SwapWindowDispatchMessageStabby> for SwapWindowDispatchMessage {
    fn from(value: SwapWindowDispatchMessageStabby) -> Self {
        Self {
            cycle_direction: value.cycle_direction,
        }
    }
}

impl TypedMessage for SwapWindowDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SwapWindowDispatchMessage");
}

impl TypedMessage for SwapWindowDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SwapWindowDispatchMessageStabby");
}

impl MessageTopic for SwapWindowDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for SwapWindowDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for SwapWindowDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
