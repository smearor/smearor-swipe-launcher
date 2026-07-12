use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Sets the orientation of the active window to left.
#[derive(Clone, Debug, Default)]
pub struct OrientationLeftDispatchMessage;

/// ABI-stable version of `OrientationLeftDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct OrientationLeftDispatchMessageStabby;

impl From<OrientationLeftDispatchMessage> for OrientationLeftDispatchMessageStabby {
    fn from(_value: OrientationLeftDispatchMessage) -> Self {
        Self
    }
}

impl From<OrientationLeftDispatchMessageStabby> for OrientationLeftDispatchMessage {
    fn from(_value: OrientationLeftDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for OrientationLeftDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OrientationLeftDispatchMessage");
}

impl TypedMessage for OrientationLeftDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OrientationLeftDispatchMessageStabby");
}

impl MessageTopic for OrientationLeftDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for OrientationLeftDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for OrientationLeftDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
