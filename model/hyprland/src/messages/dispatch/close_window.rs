use crate::HyprlandWindowIdentifier;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Closes the specified window.
#[derive(Clone, Debug, Default)]
pub struct CloseWindowDispatchMessage {
    pub window_identifier: HyprlandWindowIdentifier,
}

/// ABI-stable version of `CloseWindowDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct CloseWindowDispatchMessageStabby {
    pub window_identifier: HyprlandWindowIdentifier,
}

impl From<CloseWindowDispatchMessage> for CloseWindowDispatchMessageStabby {
    fn from(value: CloseWindowDispatchMessage) -> Self {
        Self {
            window_identifier: value.window_identifier,
        }
    }
}

impl From<CloseWindowDispatchMessageStabby> for CloseWindowDispatchMessage {
    fn from(value: CloseWindowDispatchMessageStabby) -> Self {
        Self {
            window_identifier: value.window_identifier,
        }
    }
}

impl TypedMessage for CloseWindowDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::CloseWindowDispatchMessage");
}

impl TypedMessage for CloseWindowDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::CloseWindowDispatchMessageStabby");
}

impl MessageTopic for CloseWindowDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for CloseWindowDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for CloseWindowDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
