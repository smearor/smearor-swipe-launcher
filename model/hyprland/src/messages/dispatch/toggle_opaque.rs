use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Toggles opaque state for the active window.
#[derive(Clone, Debug, Default)]
pub struct ToggleOpaqueDispatchMessage;

/// ABI-stable version of `ToggleOpaqueDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ToggleOpaqueDispatchMessageStabby;

impl From<ToggleOpaqueDispatchMessage> for ToggleOpaqueDispatchMessageStabby {
    fn from(_value: ToggleOpaqueDispatchMessage) -> Self {
        Self
    }
}

impl From<ToggleOpaqueDispatchMessageStabby> for ToggleOpaqueDispatchMessage {
    fn from(_value: ToggleOpaqueDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for ToggleOpaqueDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleOpaqueDispatchMessage");
}

impl TypedMessage for ToggleOpaqueDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleOpaqueDispatchMessageStabby");
}

impl MessageTopic for ToggleOpaqueDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for ToggleOpaqueDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ToggleOpaqueDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
