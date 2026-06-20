use crate::HyprlandFullscreenType;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Toggles fullscreen for the active window.
#[derive(Clone, Debug, Default)]
pub struct ToggleFullscreenDispatchMessage {
    pub fullscreen_type: HyprlandFullscreenType,
}

/// ABI-stable version of `ToggleFullscreenDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ToggleFullscreenDispatchMessageStabby {
    pub fullscreen_type: HyprlandFullscreenType,
}

impl From<ToggleFullscreenDispatchMessage> for ToggleFullscreenDispatchMessageStabby {
    fn from(value: ToggleFullscreenDispatchMessage) -> Self {
        Self {
            fullscreen_type: value.fullscreen_type,
        }
    }
}

impl From<ToggleFullscreenDispatchMessageStabby> for ToggleFullscreenDispatchMessage {
    fn from(value: ToggleFullscreenDispatchMessageStabby) -> Self {
        Self {
            fullscreen_type: value.fullscreen_type,
        }
    }
}

impl TypedMessage for ToggleFullscreenDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleFullscreenDispatchMessage");
}

impl TypedMessage for ToggleFullscreenDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleFullscreenDispatchMessageStabby");
}

impl MessageTopic for ToggleFullscreenDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for ToggleFullscreenDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ToggleFullscreenDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
