use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandPosition;
use crate::HyprlandWindowIdentifier;

use super::workspace::TOPIC_DISPATCH;

/// Moves a specific window by pixel position delta or to an exact position.
#[derive(Clone, Debug, Default)]
pub struct MoveWindowPixelDispatchMessage {
    pub position: HyprlandPosition,
    pub window_identifier: HyprlandWindowIdentifier,
}

/// ABI-stable version of `MoveWindowPixelDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct MoveWindowPixelDispatchMessageStabby {
    pub position: HyprlandPosition,
    pub window_identifier: HyprlandWindowIdentifier,
}

impl From<MoveWindowPixelDispatchMessage> for MoveWindowPixelDispatchMessageStabby {
    fn from(value: MoveWindowPixelDispatchMessage) -> Self {
        Self {
            position: value.position,
            window_identifier: value.window_identifier,
        }
    }
}

impl From<MoveWindowPixelDispatchMessageStabby> for MoveWindowPixelDispatchMessage {
    fn from(value: MoveWindowPixelDispatchMessageStabby) -> Self {
        Self {
            position: value.position,
            window_identifier: value.window_identifier,
        }
    }
}

impl TypedMessage for MoveWindowPixelDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveWindowPixelDispatchMessage");
}

impl TypedMessage for MoveWindowPixelDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveWindowPixelDispatchMessageStabby");
}

impl MessageTopic for MoveWindowPixelDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for MoveWindowPixelDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveWindowPixelDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
