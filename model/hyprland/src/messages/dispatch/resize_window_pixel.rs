use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandPosition;
use crate::HyprlandWindowIdentifier;

use super::workspace::TOPIC_DISPATCH;

/// Resizes a specific window by pixel position delta or to an exact position.
#[derive(Clone, Debug, Default)]
pub struct ResizeWindowPixelDispatchMessage {
    pub position: HyprlandPosition,
    pub window_identifier: HyprlandWindowIdentifier,
}

/// ABI-stable version of `ResizeWindowPixelDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ResizeWindowPixelDispatchMessageStabby {
    pub position: HyprlandPosition,
    pub window_identifier: HyprlandWindowIdentifier,
}

impl From<ResizeWindowPixelDispatchMessage> for ResizeWindowPixelDispatchMessageStabby {
    fn from(value: ResizeWindowPixelDispatchMessage) -> Self {
        Self {
            position: value.position,
            window_identifier: value.window_identifier,
        }
    }
}

impl From<ResizeWindowPixelDispatchMessageStabby> for ResizeWindowPixelDispatchMessage {
    fn from(value: ResizeWindowPixelDispatchMessageStabby) -> Self {
        Self {
            position: value.position,
            window_identifier: value.window_identifier,
        }
    }
}

impl TypedMessage for ResizeWindowPixelDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ResizeWindowPixelDispatchMessage");
}

impl TypedMessage for ResizeWindowPixelDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ResizeWindowPixelDispatchMessageStabby");
}

impl MessageTopic for ResizeWindowPixelDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for ResizeWindowPixelDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ResizeWindowPixelDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
