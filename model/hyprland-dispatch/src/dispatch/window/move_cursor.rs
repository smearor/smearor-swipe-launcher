use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Moves the cursor to the specified coordinates.
#[stabby::stabby(no_opt)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveCursorDispatchMessage {
    pub x: i64,
    pub y: i64,
}

impl TypedMessage for MoveCursorDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveCursorDispatchMessage");
}

impl MessageTopic for MoveCursorDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveCursorDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
