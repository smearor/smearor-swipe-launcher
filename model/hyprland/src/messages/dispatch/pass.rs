use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandWindowIdentifier;

use super::workspace::TOPIC_DISPATCH;

/// Passes a key press to the specified window.
#[derive(Clone, Debug, Default)]
pub struct PassDispatchMessage {
    pub window_identifier: HyprlandWindowIdentifier,
}

/// ABI-stable version of `PassDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct PassDispatchMessageStabby {
    pub window_identifier: HyprlandWindowIdentifier,
}

impl From<PassDispatchMessage> for PassDispatchMessageStabby {
    fn from(value: PassDispatchMessage) -> Self {
        Self {
            window_identifier: value.window_identifier,
        }
    }
}

impl From<PassDispatchMessageStabby> for PassDispatchMessage {
    fn from(value: PassDispatchMessageStabby) -> Self {
        Self {
            window_identifier: value.window_identifier,
        }
    }
}

impl TypedMessage for PassDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::PassDispatchMessage");
}

impl TypedMessage for PassDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::PassDispatchMessageStabby");
}

impl MessageTopic for PassDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for PassDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for PassDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
