use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Sets the cursor theme and size.
#[derive(Clone, Debug, Default)]
pub struct SetCursorDispatchMessage {
    pub theme: String,
    pub size: u16,
}

/// ABI-stable version of `SetCursorDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct SetCursorDispatchMessageStabby {
    pub theme: stabby::string::String,
    pub size: u16,
}

impl From<SetCursorDispatchMessage> for SetCursorDispatchMessageStabby {
    fn from(value: SetCursorDispatchMessage) -> Self {
        Self {
            theme: value.theme.into(),
            size: value.size,
        }
    }
}

impl From<SetCursorDispatchMessageStabby> for SetCursorDispatchMessage {
    fn from(value: SetCursorDispatchMessageStabby) -> Self {
        Self {
            theme: value.theme.to_string(),
            size: value.size,
        }
    }
}

impl TypedMessage for SetCursorDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SetCursorDispatchMessage");
}

impl TypedMessage for SetCursorDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SetCursorDispatchMessageStabby");
}

impl MessageTopic for SetCursorDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for SetCursorDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for SetCursorDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
