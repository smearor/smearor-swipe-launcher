use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Toggles the group state for the active window.
#[derive(Clone, Debug, Default)]
pub struct ToggleGroupDispatchMessage;

/// ABI-stable version of `ToggleGroupDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ToggleGroupDispatchMessageStabby;

impl From<ToggleGroupDispatchMessage> for ToggleGroupDispatchMessageStabby {
    fn from(_value: ToggleGroupDispatchMessage) -> Self {
        Self
    }
}

impl From<ToggleGroupDispatchMessageStabby> for ToggleGroupDispatchMessage {
    fn from(_value: ToggleGroupDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for ToggleGroupDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleGroupDispatchMessage");
}

impl TypedMessage for ToggleGroupDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleGroupDispatchMessageStabby");
}

impl MessageTopic for ToggleGroupDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for ToggleGroupDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ToggleGroupDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
