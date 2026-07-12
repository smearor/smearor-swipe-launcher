use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::kill::TOPIC_CTL;

/// Sets the cursor theme and size.
#[derive(Clone, Debug, Default)]
pub struct SetCursorCommandMessage {
    /// The cursor theme name.
    pub theme: String,
    /// The cursor size in pixels.
    pub size: u16,
}

/// ABI-stable version of `SetCursorCommandMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct SetCursorCommandMessageStabby {
    /// The cursor theme name.
    pub theme: stabby::string::String,
    /// The cursor size in pixels.
    pub size: u16,
}

impl From<SetCursorCommandMessage> for SetCursorCommandMessageStabby {
    fn from(value: SetCursorCommandMessage) -> Self {
        Self {
            theme: value.theme.into(),
            size: value.size,
        }
    }
}

impl From<SetCursorCommandMessageStabby> for SetCursorCommandMessage {
    fn from(value: SetCursorCommandMessageStabby) -> Self {
        Self {
            theme: value.theme.to_string(),
            size: value.size,
        }
    }
}

impl TypedMessage for SetCursorCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SetCursorCommandMessage");
}

impl TypedMessage for SetCursorCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SetCursorCommandMessageStabby");
}

impl MessageTopic for SetCursorCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl MessageTopic for SetCursorCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for SetCursorCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
