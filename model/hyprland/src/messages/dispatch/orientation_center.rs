use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Sets the orientation of the active window to center.
#[derive(Clone, Debug, Default)]
pub struct OrientationCenterDispatchMessage;

/// ABI-stable version of `OrientationCenterDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct OrientationCenterDispatchMessageStabby;

impl From<OrientationCenterDispatchMessage> for OrientationCenterDispatchMessageStabby {
    fn from(_value: OrientationCenterDispatchMessage) -> Self {
        Self
    }
}

impl From<OrientationCenterDispatchMessageStabby> for OrientationCenterDispatchMessage {
    fn from(_value: OrientationCenterDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for OrientationCenterDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OrientationCenterDispatchMessage");
}

impl TypedMessage for OrientationCenterDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OrientationCenterDispatchMessageStabby");
}

impl MessageTopic for OrientationCenterDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for OrientationCenterDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for OrientationCenterDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
