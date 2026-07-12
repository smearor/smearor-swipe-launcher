use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Moves the active window out of its group.
#[derive(Clone, Debug, Default)]
pub struct MoveOutOfGroupDispatchMessage;

/// ABI-stable version of `MoveOutOfGroupDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct MoveOutOfGroupDispatchMessageStabby;

impl From<MoveOutOfGroupDispatchMessage> for MoveOutOfGroupDispatchMessageStabby {
    fn from(_value: MoveOutOfGroupDispatchMessage) -> Self {
        Self
    }
}

impl From<MoveOutOfGroupDispatchMessageStabby> for MoveOutOfGroupDispatchMessage {
    fn from(_value: MoveOutOfGroupDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for MoveOutOfGroupDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveOutOfGroupDispatchMessage");
}

impl TypedMessage for MoveOutOfGroupDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveOutOfGroupDispatchMessageStabby");
}

impl MessageTopic for MoveOutOfGroupDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for MoveOutOfGroupDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveOutOfGroupDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
