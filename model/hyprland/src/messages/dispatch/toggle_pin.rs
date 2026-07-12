use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Toggles pin state for the active window.
#[derive(Clone, Debug, Default)]
pub struct TogglePinDispatchMessage;

/// ABI-stable version of `TogglePinDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct TogglePinDispatchMessageStabby;

impl From<TogglePinDispatchMessage> for TogglePinDispatchMessageStabby {
    fn from(_value: TogglePinDispatchMessage) -> Self {
        Self
    }
}

impl From<TogglePinDispatchMessageStabby> for TogglePinDispatchMessage {
    fn from(_value: TogglePinDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for TogglePinDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::TogglePinDispatchMessage");
}

impl TypedMessage for TogglePinDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::TogglePinDispatchMessageStabby");
}

impl MessageTopic for TogglePinDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for TogglePinDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for TogglePinDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
