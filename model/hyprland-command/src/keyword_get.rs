use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::kill::TOPIC_CTL;

/// Gets a Hyprland configuration keyword value.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KeywordGetCommandMessage {
    /// The configuration keyword name (e.g. "general:gaps_in")
    pub keyword: String,
    /// Correlation ID used to match the response back to the caller
    pub correlation_id: String,
}

/// ABI-stable version of `KeywordGetCommandMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KeywordGetCommandMessageStabby {
    /// The configuration keyword name (e.g. "general:gaps_in")
    pub keyword: stabby::string::String,
    /// Correlation ID used to match the response back to the caller
    pub correlation_id: stabby::string::String,
}

impl From<KeywordGetCommandMessage> for KeywordGetCommandMessageStabby {
    fn from(value: KeywordGetCommandMessage) -> Self {
        Self {
            keyword: value.keyword.into(),
            correlation_id: value.correlation_id.into(),
        }
    }
}

impl From<KeywordGetCommandMessageStabby> for KeywordGetCommandMessage {
    fn from(value: KeywordGetCommandMessageStabby) -> Self {
        Self {
            keyword: value.keyword.to_string(),
            correlation_id: value.correlation_id.to_string(),
        }
    }
}

impl TypedMessage for KeywordGetCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::KeywordGetCommandMessage");
}

impl TypedMessage for KeywordGetCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::KeywordGetCommandMessageStabby");
}

impl MessageTopic for KeywordGetCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl MessageTopic for KeywordGetCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for KeywordGetCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
