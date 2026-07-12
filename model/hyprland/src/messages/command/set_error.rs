use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::kill::TOPIC_CTL;
use crate::HyprlandColor;

/// Creates an error that Hyprland will display.
#[derive(Clone, Debug, Default)]
pub struct SetErrorCommandMessage {
    /// The color of the error message.
    pub color: HyprlandColor,
    /// The error message text.
    pub message: String,
}

/// ABI-stable version of `SetErrorCommandMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct SetErrorCommandMessageStabby {
    /// The color of the error message.
    pub color: HyprlandColor,
    /// The error message text.
    pub message: stabby::string::String,
}

impl From<SetErrorCommandMessage> for SetErrorCommandMessageStabby {
    fn from(value: SetErrorCommandMessage) -> Self {
        Self {
            color: value.color,
            message: value.message.into(),
        }
    }
}

impl From<SetErrorCommandMessageStabby> for SetErrorCommandMessage {
    fn from(value: SetErrorCommandMessageStabby) -> Self {
        Self {
            color: value.color,
            message: value.message.to_string(),
        }
    }
}

impl TypedMessage for SetErrorCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SetErrorCommandMessage");
}

impl TypedMessage for SetErrorCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SetErrorCommandMessageStabby");
}

impl MessageTopic for SetErrorCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl MessageTopic for SetErrorCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for SetErrorCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
