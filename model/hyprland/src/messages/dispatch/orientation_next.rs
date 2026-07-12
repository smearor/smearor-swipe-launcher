use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Sets the orientation of the active window to the next orientation.
#[derive(Clone, Debug, Default)]
pub struct OrientationNextDispatchMessage;

/// ABI-stable version of `OrientationNextDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct OrientationNextDispatchMessageStabby;

impl From<OrientationNextDispatchMessage> for OrientationNextDispatchMessageStabby {
    fn from(_value: OrientationNextDispatchMessage) -> Self {
        Self
    }
}

impl From<OrientationNextDispatchMessageStabby> for OrientationNextDispatchMessage {
    fn from(_value: OrientationNextDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for OrientationNextDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OrientationNextDispatchMessage");
}

impl TypedMessage for OrientationNextDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OrientationNextDispatchMessageStabby");
}

impl MessageTopic for OrientationNextDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for OrientationNextDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for OrientationNextDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
