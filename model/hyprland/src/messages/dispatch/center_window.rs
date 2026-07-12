use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Centers the active window on screen.
#[derive(Clone, Debug, Default)]
pub struct CenterWindowDispatchMessage;

/// ABI-stable version of `CenterWindowDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct CenterWindowDispatchMessageStabby;

impl From<CenterWindowDispatchMessage> for CenterWindowDispatchMessageStabby {
    fn from(_value: CenterWindowDispatchMessage) -> Self {
        Self
    }
}

impl From<CenterWindowDispatchMessageStabby> for CenterWindowDispatchMessage {
    fn from(_value: CenterWindowDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for CenterWindowDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::CenterWindowDispatchMessage");
}

impl TypedMessage for CenterWindowDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::CenterWindowDispatchMessageStabby");
}

impl MessageTopic for CenterWindowDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for CenterWindowDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for CenterWindowDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
