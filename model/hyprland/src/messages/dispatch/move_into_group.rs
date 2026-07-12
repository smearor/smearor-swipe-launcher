use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandDirection;

use super::workspace::TOPIC_DISPATCH;

/// Moves the active window into a group in the specified direction.
#[derive(Clone, Debug, Default)]
pub struct MoveIntoGroupDispatchMessage {
    pub direction: HyprlandDirection,
}

/// ABI-stable version of `MoveIntoGroupDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct MoveIntoGroupDispatchMessageStabby {
    pub direction: HyprlandDirection,
}

impl From<MoveIntoGroupDispatchMessage> for MoveIntoGroupDispatchMessageStabby {
    fn from(value: MoveIntoGroupDispatchMessage) -> Self {
        Self { direction: value.direction }
    }
}

impl From<MoveIntoGroupDispatchMessageStabby> for MoveIntoGroupDispatchMessage {
    fn from(value: MoveIntoGroupDispatchMessageStabby) -> Self {
        Self { direction: value.direction }
    }
}

impl TypedMessage for MoveIntoGroupDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveIntoGroupDispatchMessage");
}

impl TypedMessage for MoveIntoGroupDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveIntoGroupDispatchMessageStabby");
}

impl MessageTopic for MoveIntoGroupDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for MoveIntoGroupDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveIntoGroupDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
