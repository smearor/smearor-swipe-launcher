use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::kill::TOPIC_CTL;

/// Sets a Hyprland configuration keyword at runtime.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KeywordSetCommandMessage {
    /// The configuration keyword name (e.g. "general:gaps_in")
    pub keyword: String,
    /// The value to set (as a string; Hyprland parses int/float/str)
    pub value: String,
}

/// ABI-stable version of `KeywordSetCommandMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KeywordSetCommandMessageStabby {
    /// The configuration keyword name (e.g. "general:gaps_in")
    pub keyword: stabby::string::String,
    /// The value to set (as a string; Hyprland parses int/float/str)
    pub value: stabby::string::String,
}

impl From<KeywordSetCommandMessage> for KeywordSetCommandMessageStabby {
    fn from(value: KeywordSetCommandMessage) -> Self {
        Self {
            keyword: value.keyword.into(),
            value: value.value.into(),
        }
    }
}

impl From<KeywordSetCommandMessageStabby> for KeywordSetCommandMessage {
    fn from(value: KeywordSetCommandMessageStabby) -> Self {
        Self {
            keyword: value.keyword.to_string(),
            value: value.value.to_string(),
        }
    }
}

impl TypedMessage for KeywordSetCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::KeywordSetCommandMessage");
}

impl TypedMessage for KeywordSetCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::KeywordSetCommandMessageStabby");
}

impl MessageTopic for KeywordSetCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl MessageTopic for KeywordSetCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for KeywordSetCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
