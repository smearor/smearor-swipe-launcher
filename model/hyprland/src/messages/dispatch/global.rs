use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Executes a global keybinding dispatch.
#[derive(Clone, Debug, Default)]
pub struct GlobalDispatchMessage {
    pub key: String,
}

/// ABI-stable version of `GlobalDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct GlobalDispatchMessageStabby {
    pub key: stabby::string::String,
}

impl From<GlobalDispatchMessage> for GlobalDispatchMessageStabby {
    fn from(value: GlobalDispatchMessage) -> Self {
        Self { key: value.key.into() }
    }
}

impl From<GlobalDispatchMessageStabby> for GlobalDispatchMessage {
    fn from(value: GlobalDispatchMessageStabby) -> Self {
        Self { key: value.key.to_string() }
    }
}

impl TypedMessage for GlobalDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::GlobalDispatchMessage");
}

impl TypedMessage for GlobalDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::GlobalDispatchMessageStabby");
}

impl MessageTopic for GlobalDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for GlobalDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for GlobalDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
