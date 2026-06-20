use crate::HyprlandDirection;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Moves focus in the given direction.
#[derive(Clone, Debug, Default)]
pub struct MoveFocusDispatchMessage {
    pub direction: HyprlandDirection,
}

/// ABI-stable version of `MoveFocusDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct MoveFocusDispatchMessageStabby {
    pub direction: HyprlandDirection,
}

impl From<MoveFocusDispatchMessage> for MoveFocusDispatchMessageStabby {
    fn from(value: MoveFocusDispatchMessage) -> Self {
        Self { direction: value.direction }
    }
}

impl From<MoveFocusDispatchMessageStabby> for MoveFocusDispatchMessage {
    fn from(value: MoveFocusDispatchMessageStabby) -> Self {
        Self { direction: value.direction }
    }
}

impl TypedMessage for MoveFocusDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveFocusDispatchMessage");
}

impl TypedMessage for MoveFocusDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveFocusDispatchMessageStabby");
}

impl MessageTopic for MoveFocusDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for MoveFocusDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveFocusDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
