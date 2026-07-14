use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::messages::topics::TOPIC_COMMAND;

/// Actions the voice assistant service can perform on request.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum VoiceCommandAction {
    /// Start audio capture and begin the voice pipeline.
    #[default]
    Activate,
    /// Stop audio capture and cancel any in-progress pipeline.
    Deactivate,
    /// Submit a text command directly (bypassing STT, e.g., from a text input).
    SubmitText,
    /// Clear conversation history and KV cache, starting a fresh session.
    ClearConversation,
}

/// Command message sent by widgets or external clients to the voice assistant service.
#[derive(Clone, Debug, Default)]
pub struct VoiceCommandMessage {
    /// The action to execute.
    pub action: VoiceCommandAction,
    /// Text input when action is SubmitText; empty for Activate/Deactivate.
    pub text: String,
}

/// ABI-stable version of `VoiceCommandMessage` for cross-plugin messaging.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct VoiceCommandMessageStabby {
    pub action: VoiceCommandAction,
    pub text: stabby::string::String,
}

impl From<VoiceCommandMessage> for VoiceCommandMessageStabby {
    fn from(value: VoiceCommandMessage) -> Self {
        Self {
            action: value.action,
            text: value.text.into(),
        }
    }
}

impl From<VoiceCommandMessageStabby> for VoiceCommandMessage {
    fn from(value: VoiceCommandMessageStabby) -> Self {
        Self {
            action: value.action,
            text: value.text.to_string(),
        }
    }
}

impl VoiceCommandMessage {
    pub fn new(action: VoiceCommandAction, text: &str) -> Self {
        Self {
            action,
            text: text.to_string(),
        }
    }

    pub fn activate() -> Self {
        Self::new(VoiceCommandAction::Activate, "")
    }

    pub fn deactivate() -> Self {
        Self::new(VoiceCommandAction::Deactivate, "")
    }

    pub fn submit_text(text: &str) -> Self {
        Self::new(VoiceCommandAction::SubmitText, text)
    }

    pub fn clear_conversation() -> Self {
        Self::new(VoiceCommandAction::ClearConversation, "")
    }
}

impl TypedMessage for VoiceCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_voice_assistant_model::VoiceCommandMessageStabby");
}

impl TypedMessage for VoiceCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_voice_assistant_model::VoiceCommandMessage");
}

impl MessageTopic for VoiceCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl MessageTopic for VoiceCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl SharedMessage for VoiceCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_COMMAND
    }
}
