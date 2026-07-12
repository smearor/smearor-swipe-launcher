use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Sets the orientation of the active window to bottom.
#[derive(Clone, Debug, Default)]
pub struct OrientationBottomDispatchMessage;

/// ABI-stable version of `OrientationBottomDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct OrientationBottomDispatchMessageStabby;

impl From<OrientationBottomDispatchMessage> for OrientationBottomDispatchMessageStabby {
    fn from(_value: OrientationBottomDispatchMessage) -> Self {
        Self
    }
}

impl From<OrientationBottomDispatchMessageStabby> for OrientationBottomDispatchMessage {
    fn from(_value: OrientationBottomDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for OrientationBottomDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OrientationBottomDispatchMessage");
}

impl TypedMessage for OrientationBottomDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OrientationBottomDispatchMessageStabby");
}

impl MessageTopic for OrientationBottomDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for OrientationBottomDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for OrientationBottomDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
