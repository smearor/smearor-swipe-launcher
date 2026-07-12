use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Sets the orientation of the active window to the previous orientation.
#[derive(Clone, Debug, Default)]
pub struct OrientationPrevDispatchMessage;

/// ABI-stable version of `OrientationPrevDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct OrientationPrevDispatchMessageStabby;

impl From<OrientationPrevDispatchMessage> for OrientationPrevDispatchMessageStabby {
    fn from(_value: OrientationPrevDispatchMessage) -> Self {
        Self
    }
}

impl From<OrientationPrevDispatchMessageStabby> for OrientationPrevDispatchMessage {
    fn from(_value: OrientationPrevDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for OrientationPrevDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OrientationPrevDispatchMessage");
}

impl TypedMessage for OrientationPrevDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OrientationPrevDispatchMessageStabby");
}

impl MessageTopic for OrientationPrevDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for OrientationPrevDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for OrientationPrevDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
