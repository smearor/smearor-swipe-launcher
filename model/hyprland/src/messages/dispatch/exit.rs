use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Exits the Hyprland compositor.
#[derive(Clone, Debug, Default)]
pub struct ExitDispatchMessage;

/// ABI-stable version of `ExitDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ExitDispatchMessageStabby;

impl From<ExitDispatchMessage> for ExitDispatchMessageStabby {
    fn from(_value: ExitDispatchMessage) -> Self {
        Self
    }
}

impl From<ExitDispatchMessageStabby> for ExitDispatchMessage {
    fn from(_value: ExitDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for ExitDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ExitDispatchMessage");
}

impl TypedMessage for ExitDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ExitDispatchMessageStabby");
}

impl MessageTopic for ExitDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for ExitDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ExitDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
