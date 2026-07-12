use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Sets the orientation of the active window to top.
#[derive(Clone, Debug, Default)]
pub struct OrientationTopDispatchMessage;

/// ABI-stable version of `OrientationTopDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct OrientationTopDispatchMessageStabby;

impl From<OrientationTopDispatchMessage> for OrientationTopDispatchMessageStabby {
    fn from(_value: OrientationTopDispatchMessage) -> Self {
        Self
    }
}

impl From<OrientationTopDispatchMessageStabby> for OrientationTopDispatchMessage {
    fn from(_value: OrientationTopDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for OrientationTopDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OrientationTopDispatchMessage");
}

impl TypedMessage for OrientationTopDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OrientationTopDispatchMessageStabby");
}

impl MessageTopic for OrientationTopDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for OrientationTopDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for OrientationTopDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
