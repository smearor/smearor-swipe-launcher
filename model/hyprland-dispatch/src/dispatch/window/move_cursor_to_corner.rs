use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandCorner;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Moves the cursor to the specified corner of the active window.
#[stabby::stabby(no_opt)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveCursorToCornerDispatchMessage {
    pub corner: HyprlandCorner,
}

impl TypedMessage for MoveCursorToCornerDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveCursorToCornerDispatchMessage");
}

impl MessageTopic for MoveCursorToCornerDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveCursorToCornerDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
