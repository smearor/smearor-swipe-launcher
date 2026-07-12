use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Moves the cursor to the specified coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoveCursorDispatchMessage {
    pub x: i64,
    pub y: i64,
}

/// ABI-stable version of `MoveCursorDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoveCursorDispatchMessageStabby {
    pub x: i64,
    pub y: i64,
}

impl From<MoveCursorDispatchMessage> for MoveCursorDispatchMessageStabby {
    fn from(value: MoveCursorDispatchMessage) -> Self {
        Self { x: value.x, y: value.y }
    }
}

impl From<MoveCursorDispatchMessageStabby> for MoveCursorDispatchMessage {
    fn from(value: MoveCursorDispatchMessageStabby) -> Self {
        Self { x: value.x, y: value.y }
    }
}

impl TypedMessage for MoveCursorDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveCursorDispatchMessage");
}

impl TypedMessage for MoveCursorDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveCursorDispatchMessageStabby");
}

impl MessageTopic for MoveCursorDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for MoveCursorDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveCursorDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
