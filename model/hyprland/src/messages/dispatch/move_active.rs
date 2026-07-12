use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandPosition;

use super::workspace::TOPIC_DISPATCH;

/// Moves the active window by the given position delta or to an exact position.
#[derive(Clone, Debug, Default)]
pub struct MoveActiveDispatchMessage {
    pub position: HyprlandPosition,
}

/// ABI-stable version of `MoveActiveDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct MoveActiveDispatchMessageStabby {
    pub position: HyprlandPosition,
}

impl From<MoveActiveDispatchMessage> for MoveActiveDispatchMessageStabby {
    fn from(value: MoveActiveDispatchMessage) -> Self {
        Self { position: value.position }
    }
}

impl From<MoveActiveDispatchMessageStabby> for MoveActiveDispatchMessage {
    fn from(value: MoveActiveDispatchMessageStabby) -> Self {
        Self { position: value.position }
    }
}

impl TypedMessage for MoveActiveDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveActiveDispatchMessage");
}

impl TypedMessage for MoveActiveDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveActiveDispatchMessageStabby");
}

impl MessageTopic for MoveActiveDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for MoveActiveDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveActiveDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
