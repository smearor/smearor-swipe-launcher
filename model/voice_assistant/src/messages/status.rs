use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::messages::state::AssistantState;
use crate::messages::topics::TOPIC_STATUS;

/// Status message broadcast by the voice assistant service.
/// Contains the current pipeline state, partial transcription, and optional error details.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AssistantStatusMessage {
    /// Current pipeline state.
    pub current_state: AssistantState,
    /// Partial or complete transcription of the user's speech.
    pub partial_transcript: String,
    /// The last final answer produced by the LLM (if any).
    pub final_answer: Option<String>,
    /// The tool currently being executed (if in ExecutingAction state).
    pub active_tool: Option<String>,
    /// Error message when current_state is Error.
    pub error_message: Option<String>,
    /// The type of the LLM response (e.g., "final_answer", "clarify") for MCP clients.
    pub response_type: Option<String>,
}

/// ABI-stable version of `AssistantStatusMessage` for cross-plugin messaging.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AssistantStatusMessageStabby {
    pub current_state: AssistantState,
    pub partial_transcript: stabby::string::String,
    pub final_answer: stabby::option::Option<stabby::string::String>,
    pub active_tool: stabby::option::Option<stabby::string::String>,
    pub error_message: stabby::option::Option<stabby::string::String>,
    pub response_type: stabby::option::Option<stabby::string::String>,
}

impl From<AssistantStatusMessage> for AssistantStatusMessageStabby {
    fn from(value: AssistantStatusMessage) -> Self {
        Self {
            current_state: value.current_state,
            partial_transcript: value.partial_transcript.into(),
            final_answer: value.final_answer.map(Into::into).into(),
            active_tool: value.active_tool.map(Into::into).into(),
            error_message: value.error_message.map(Into::into).into(),
            response_type: value.response_type.map(Into::into).into(),
        }
    }
}

impl From<AssistantStatusMessageStabby> for AssistantStatusMessage {
    fn from(value: AssistantStatusMessageStabby) -> Self {
        Self {
            current_state: value.current_state,
            partial_transcript: value.partial_transcript.to_string(),
            final_answer: {
                let opt: Option<stabby::string::String> = value.final_answer.into();
                opt.map(|s| s.to_string())
            },
            active_tool: {
                let opt: Option<stabby::string::String> = value.active_tool.into();
                opt.map(|s| s.to_string())
            },
            error_message: {
                let opt: Option<stabby::string::String> = value.error_message.into();
                opt.map(|s| s.to_string())
            },
            response_type: {
                let opt: Option<stabby::string::String> = value.response_type.into();
                opt.map(|s| s.to_string())
            },
        }
    }
}

impl AssistantStatusMessage {
    pub fn new(current_state: AssistantState) -> Self {
        Self {
            current_state,
            partial_transcript: String::new(),
            final_answer: None,
            active_tool: None,
            error_message: None,
            response_type: None,
        }
    }

    pub fn with_transcript(mut self, transcript: &str) -> Self {
        self.partial_transcript = transcript.to_string();
        self
    }

    pub fn with_final_answer(mut self, answer: &str) -> Self {
        self.final_answer = Some(answer.to_string());
        self
    }

    pub fn with_active_tool(mut self, tool: &str) -> Self {
        self.active_tool = Some(tool.to_string());
        self
    }

    pub fn with_error(mut self, error: &str) -> Self {
        self.error_message = Some(error.to_string());
        self
    }

    pub fn with_response_type(mut self, response_type: &str) -> Self {
        self.response_type = Some(response_type.to_string());
        self
    }
}

impl TypedMessage for AssistantStatusMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_voice_assistant_model::AssistantStatusMessageStabby");
}

impl TypedMessage for AssistantStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_voice_assistant_model::AssistantStatusMessage");
}

impl MessageTopic for AssistantStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl MessageTopic for AssistantStatusMessageStabby {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl SharedMessage for AssistantStatusMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_STATUS
    }
}
