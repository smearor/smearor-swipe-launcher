use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Toggles fake fullscreen state for the active window.
#[derive(Clone, Debug, Default)]
pub struct ToggleFakeFullscreenDispatchMessage;

/// ABI-stable version of `ToggleFakeFullscreenDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ToggleFakeFullscreenDispatchMessageStabby;

impl From<ToggleFakeFullscreenDispatchMessage> for ToggleFakeFullscreenDispatchMessageStabby {
    fn from(_value: ToggleFakeFullscreenDispatchMessage) -> Self {
        Self
    }
}

impl From<ToggleFakeFullscreenDispatchMessageStabby> for ToggleFakeFullscreenDispatchMessage {
    fn from(_value: ToggleFakeFullscreenDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for ToggleFakeFullscreenDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleFakeFullscreenDispatchMessage");
}

impl TypedMessage for ToggleFakeFullscreenDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleFakeFullscreenDispatchMessageStabby");
}

impl MessageTopic for ToggleFakeFullscreenDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for ToggleFakeFullscreenDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ToggleFakeFullscreenDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
