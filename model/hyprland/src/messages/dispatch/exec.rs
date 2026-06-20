use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Executes an arbitrary command.
#[derive(Clone, Debug, Default)]
pub struct ExecDispatchMessage {
    pub command: String,
}

/// ABI-stable version of `ExecDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ExecDispatchMessageStabby {
    pub command: stabby::string::String,
}

impl From<ExecDispatchMessage> for ExecDispatchMessageStabby {
    fn from(value: ExecDispatchMessage) -> Self {
        Self { command: value.command.into() }
    }
}

impl From<ExecDispatchMessageStabby> for ExecDispatchMessage {
    fn from(value: ExecDispatchMessageStabby) -> Self {
        Self {
            command: value.command.to_string(),
        }
    }
}

impl TypedMessage for ExecDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ExecDispatchMessage");
}

impl TypedMessage for ExecDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ExecDispatchMessageStabby");
}

impl MessageTopic for ExecDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for ExecDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ExecDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
