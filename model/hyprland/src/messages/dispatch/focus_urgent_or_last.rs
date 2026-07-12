use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Focuses the urgent window or the last focused window.
#[derive(Clone, Debug, Default)]
pub struct FocusUrgentOrLastDispatchMessage;

/// ABI-stable version of `FocusUrgentOrLastDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct FocusUrgentOrLastDispatchMessageStabby;

impl From<FocusUrgentOrLastDispatchMessage> for FocusUrgentOrLastDispatchMessageStabby {
    fn from(_value: FocusUrgentOrLastDispatchMessage) -> Self {
        Self
    }
}

impl From<FocusUrgentOrLastDispatchMessageStabby> for FocusUrgentOrLastDispatchMessage {
    fn from(_value: FocusUrgentOrLastDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for FocusUrgentOrLastDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FocusUrgentOrLastDispatchMessage");
}

impl TypedMessage for FocusUrgentOrLastDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FocusUrgentOrLastDispatchMessageStabby");
}

impl MessageTopic for FocusUrgentOrLastDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for FocusUrgentOrLastDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for FocusUrgentOrLastDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
