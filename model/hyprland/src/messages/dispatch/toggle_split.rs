use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Toggles the split orientation of the active window.
#[derive(Clone, Debug, Default)]
pub struct ToggleSplitDispatchMessage;

/// ABI-stable version of `ToggleSplitDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ToggleSplitDispatchMessageStabby;

impl From<ToggleSplitDispatchMessage> for ToggleSplitDispatchMessageStabby {
    fn from(_value: ToggleSplitDispatchMessage) -> Self {
        Self
    }
}

impl From<ToggleSplitDispatchMessageStabby> for ToggleSplitDispatchMessage {
    fn from(_value: ToggleSplitDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for ToggleSplitDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleSplitDispatchMessage");
}

impl TypedMessage for ToggleSplitDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleSplitDispatchMessageStabby");
}

impl MessageTopic for ToggleSplitDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for ToggleSplitDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ToggleSplitDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
