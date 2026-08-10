use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::kill::TOPIC_CTL;

/// Sends a keyboard shortcut to a window via Hyprland dispatch.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SendShortcutCommandMessage {
    /// Modifier keys as a comma-separated string (e.g. "SUPER", "CTRL,SHIFT")
    pub mods: String,
    /// The key name (e.g. "S", "F1", "space")
    pub key: String,
    /// Optional window identifier (e.g. "address:0x1234"). If None, sends to active window.
    pub window: Option<String>,
}

/// ABI-stable version of `SendShortcutCommandMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SendShortcutCommandMessageStabby {
    /// Modifier keys as a comma-separated string (e.g. "SUPER", "CTRL,SHIFT")
    pub mods: stabby::string::String,
    /// The key name (e.g. "S", "F1", "space")
    pub key: stabby::string::String,
    /// Optional window identifier (e.g. "address:0x1234"). If None, sends to active window.
    pub window: stabby::option::Option<stabby::string::String>,
}

impl From<SendShortcutCommandMessage> for SendShortcutCommandMessageStabby {
    fn from(value: SendShortcutCommandMessage) -> Self {
        Self {
            mods: value.mods.into(),
            key: value.key.into(),
            window: value.window.map(stabby::string::String::from).into(),
        }
    }
}

impl From<SendShortcutCommandMessageStabby> for SendShortcutCommandMessage {
    fn from(value: SendShortcutCommandMessageStabby) -> Self {
        let window: Option<stabby::string::String> = value.window.into();
        Self {
            mods: value.mods.to_string(),
            key: value.key.to_string(),
            window: window.map(|s| s.to_string()),
        }
    }
}

impl TypedMessage for SendShortcutCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SendShortcutCommandMessage");
}

impl TypedMessage for SendShortcutCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SendShortcutCommandMessageStabby");
}

impl MessageTopic for SendShortcutCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl MessageTopic for SendShortcutCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for SendShortcutCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
