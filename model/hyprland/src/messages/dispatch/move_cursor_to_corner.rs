use crate::HyprlandCorner;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Moves the cursor to the specified corner of the active window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoveCursorToCornerDispatchMessage {
    pub corner: HyprlandCorner,
}

/// ABI-stable version of `MoveCursorToCornerDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoveCursorToCornerDispatchMessageStabby {
    pub corner: HyprlandCorner,
}

impl From<MoveCursorToCornerDispatchMessage> for MoveCursorToCornerDispatchMessageStabby {
    fn from(value: MoveCursorToCornerDispatchMessage) -> Self {
        Self { corner: value.corner }
    }
}

impl From<MoveCursorToCornerDispatchMessageStabby> for MoveCursorToCornerDispatchMessage {
    fn from(value: MoveCursorToCornerDispatchMessageStabby) -> Self {
        Self { corner: value.corner }
    }
}

impl TypedMessage for MoveCursorToCornerDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveCursorToCornerDispatchMessage");
}

impl TypedMessage for MoveCursorToCornerDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveCursorToCornerDispatchMessageStabby");
}

impl MessageTopic for MoveCursorToCornerDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for MoveCursorToCornerDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveCursorToCornerDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
