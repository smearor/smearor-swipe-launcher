use crate::HyprlandWindowIdentifier;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Focuses the specified window.
#[derive(Clone, Debug, Default)]
pub struct FocusWindowDispatchMessage {
    pub window_identifier: HyprlandWindowIdentifier,
}

/// ABI-stable version of `FocusWindowDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct FocusWindowDispatchMessageStabby {
    pub window_identifier: HyprlandWindowIdentifier,
}

impl From<FocusWindowDispatchMessage> for FocusWindowDispatchMessageStabby {
    fn from(value: FocusWindowDispatchMessage) -> Self {
        Self {
            window_identifier: value.window_identifier,
        }
    }
}

impl From<FocusWindowDispatchMessageStabby> for FocusWindowDispatchMessage {
    fn from(value: FocusWindowDispatchMessageStabby) -> Self {
        Self {
            window_identifier: value.window_identifier,
        }
    }
}

impl TypedMessage for FocusWindowDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FocusWindowDispatchMessage");
}

impl TypedMessage for FocusWindowDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FocusWindowDispatchMessageStabby");
}

impl MessageTopic for FocusWindowDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for FocusWindowDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for FocusWindowDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
