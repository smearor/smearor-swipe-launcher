use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandWindowMove;

use super::workspace::TOPIC_DISPATCH;

/// Moves the active window in the specified direction or to a monitor.
#[derive(Clone, Debug, Default)]
pub struct MoveWindowDispatchMessage {
    pub window_move: HyprlandWindowMove,
}

/// ABI-stable version of `MoveWindowDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct MoveWindowDispatchMessageStabby {
    pub window_move: HyprlandWindowMove,
}

impl From<MoveWindowDispatchMessage> for MoveWindowDispatchMessageStabby {
    fn from(value: MoveWindowDispatchMessage) -> Self {
        Self {
            window_move: value.window_move,
        }
    }
}

impl From<MoveWindowDispatchMessageStabby> for MoveWindowDispatchMessage {
    fn from(value: MoveWindowDispatchMessageStabby) -> Self {
        Self {
            window_move: value.window_move,
        }
    }
}

impl TypedMessage for MoveWindowDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveWindowDispatchMessage");
}

impl TypedMessage for MoveWindowDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveWindowDispatchMessageStabby");
}

impl MessageTopic for MoveWindowDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for MoveWindowDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveWindowDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
