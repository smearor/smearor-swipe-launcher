use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Toggles the floating state of the active window.
#[derive(Clone, Debug, Default)]
pub struct ToggleFloatingDispatchMessage;

/// ABI-stable version of `ToggleFloatingDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ToggleFloatingDispatchMessageStabby;

impl From<ToggleFloatingDispatchMessage> for ToggleFloatingDispatchMessageStabby {
    fn from(_value: ToggleFloatingDispatchMessage) -> Self {
        Self
    }
}

impl From<ToggleFloatingDispatchMessageStabby> for ToggleFloatingDispatchMessage {
    fn from(_value: ToggleFloatingDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for ToggleFloatingDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleFloatingDispatchMessage");
}

impl TypedMessage for ToggleFloatingDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleFloatingDispatchMessageStabby");
}

impl MessageTopic for ToggleFloatingDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for ToggleFloatingDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ToggleFloatingDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
