use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Closes the currently active window.
#[derive(Clone, Debug, Default)]
pub struct KillActiveWindowDispatchMessage;

/// ABI-stable version of `KillActiveWindowDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct KillActiveWindowDispatchMessageStabby;

impl From<KillActiveWindowDispatchMessage> for KillActiveWindowDispatchMessageStabby {
    fn from(_value: KillActiveWindowDispatchMessage) -> Self {
        Self
    }
}

impl From<KillActiveWindowDispatchMessageStabby> for KillActiveWindowDispatchMessage {
    fn from(_value: KillActiveWindowDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for KillActiveWindowDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::KillActiveWindowDispatchMessage");
}

impl TypedMessage for KillActiveWindowDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::KillActiveWindowDispatchMessageStabby");
}

impl MessageTopic for KillActiveWindowDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for KillActiveWindowDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for KillActiveWindowDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
