use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Toggles pseudo-tiling for the active window.
#[derive(Clone, Debug, Default)]
pub struct TogglePseudoDispatchMessage;

/// ABI-stable version of `TogglePseudoDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct TogglePseudoDispatchMessageStabby;

impl From<TogglePseudoDispatchMessage> for TogglePseudoDispatchMessageStabby {
    fn from(_value: TogglePseudoDispatchMessage) -> Self {
        Self
    }
}

impl From<TogglePseudoDispatchMessageStabby> for TogglePseudoDispatchMessage {
    fn from(_value: TogglePseudoDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for TogglePseudoDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::TogglePseudoDispatchMessage");
}

impl TypedMessage for TogglePseudoDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::TogglePseudoDispatchMessageStabby");
}

impl MessageTopic for TogglePseudoDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for TogglePseudoDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for TogglePseudoDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
