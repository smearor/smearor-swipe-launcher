use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Sets the orientation of the active window to right.
#[derive(Clone, Debug, Default)]
pub struct OrientationRightDispatchMessage;

/// ABI-stable version of `OrientationRightDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct OrientationRightDispatchMessageStabby;

impl From<OrientationRightDispatchMessage> for OrientationRightDispatchMessageStabby {
    fn from(_value: OrientationRightDispatchMessage) -> Self {
        Self
    }
}

impl From<OrientationRightDispatchMessageStabby> for OrientationRightDispatchMessage {
    fn from(_value: OrientationRightDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for OrientationRightDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OrientationRightDispatchMessage");
}

impl TypedMessage for OrientationRightDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OrientationRightDispatchMessageStabby");
}

impl MessageTopic for OrientationRightDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for OrientationRightDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for OrientationRightDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
