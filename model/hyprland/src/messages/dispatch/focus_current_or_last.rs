use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Focuses the current window or the last focused window.
#[derive(Clone, Debug, Default)]
pub struct FocusCurrentOrLastDispatchMessage;

/// ABI-stable version of `FocusCurrentOrLastDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct FocusCurrentOrLastDispatchMessageStabby;

impl From<FocusCurrentOrLastDispatchMessage> for FocusCurrentOrLastDispatchMessageStabby {
    fn from(_value: FocusCurrentOrLastDispatchMessage) -> Self {
        Self
    }
}

impl From<FocusCurrentOrLastDispatchMessageStabby> for FocusCurrentOrLastDispatchMessage {
    fn from(_value: FocusCurrentOrLastDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for FocusCurrentOrLastDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FocusCurrentOrLastDispatchMessage");
}

impl TypedMessage for FocusCurrentOrLastDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FocusCurrentOrLastDispatchMessageStabby");
}

impl MessageTopic for FocusCurrentOrLastDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for FocusCurrentOrLastDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for FocusCurrentOrLastDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
