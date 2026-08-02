use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::prompts::invoke::error::InvokePromptError;
use crate::prompts::invoke::message::PromptMessage;

pub const TOPIC_MCP_PROMPT_RESPONSE: &str = "mcp.prompt.response";

/// Response returned by a plugin after resolving a registered prompt.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct InvokePromptResponse {
    /// Correlation ID matching the request.
    pub correlation_id: stabby::string::String,
    /// Resolved prompt messages. Empty on error.
    pub messages: stabby::vec::Vec<PromptMessage>,
    /// Error message. Empty when the resolution succeeded.
    pub error: stabby::string::String,
}

impl InvokePromptResponse {
    /// Create a successful prompt response.
    pub fn success(correlation_id: &str, messages: Vec<PromptMessage>) -> Self {
        Self {
            correlation_id: correlation_id.into(),
            messages: messages.into_iter().collect(),
            error: "".into(),
        }
    }

    /// Create an error prompt response.
    pub fn error(correlation_id: &str, error: &str) -> Self {
        Self {
            correlation_id: correlation_id.into(),
            messages: stabby::vec::Vec::new(),
            error: error.into(),
        }
    }
}

impl From<InvokePromptError> for InvokePromptResponse {
    fn from(e: InvokePromptError) -> Self {
        InvokePromptResponse::error(&e.correlation_id, &format!("Unknown mcp prompt {}", e.name))
    }
}

impl TypedMessage for InvokePromptResponse {
    const TYPE_ID: u64 = generate_type_id("smearor_model_mcp::InvokePromptResponse");
}

impl MessageTopic for InvokePromptResponse {
    fn topic() -> &'static str {
        TOPIC_MCP_PROMPT_RESPONSE
    }
}

impl SharedMessage for InvokePromptResponse {
    fn topic(&self) -> &'static str {
        TOPIC_MCP_PROMPT_RESPONSE
    }
}
