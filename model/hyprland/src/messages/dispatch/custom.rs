use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Executes a custom dispatch command with a name and value.
#[derive(Clone, Debug, Default)]
pub struct CustomDispatchMessage {
    pub name: String,
    pub value: String,
}

/// ABI-stable version of `CustomDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct CustomDispatchMessageStabby {
    pub name: stabby::string::String,
    pub value: stabby::string::String,
}

impl From<CustomDispatchMessage> for CustomDispatchMessageStabby {
    fn from(value: CustomDispatchMessage) -> Self {
        Self {
            name: value.name.into(),
            value: value.value.into(),
        }
    }
}

impl From<CustomDispatchMessageStabby> for CustomDispatchMessage {
    fn from(value: CustomDispatchMessageStabby) -> Self {
        Self {
            name: value.name.to_string(),
            value: value.value.to_string(),
        }
    }
}

impl TypedMessage for CustomDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::CustomDispatchMessage");
}

impl TypedMessage for CustomDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::CustomDispatchMessageStabby");
}

impl MessageTopic for CustomDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for CustomDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for CustomDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
