use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_MCP_INVOKE_PROMPT: &str = "mcp.invoke.prompt";

/// Request sent by the host to a plugin to resolve a registered prompt.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct InvokePromptMessage {
    /// Prompt name as registered by the plugin.
    pub name: stabby::string::String,
    /// Correlation ID used to match the response.
    pub correlation_id: stabby::string::String,
    /// JSON-encoded arguments for the prompt, matching the arguments schema.
    pub arguments: stabby::string::String,
}

impl InvokePromptMessage {
    /// Create a new prompt invocation request.
    pub fn new(name: &str, correlation_id: &str, arguments: &str) -> Self {
        Self {
            name: name.into(),
            correlation_id: correlation_id.into(),
            arguments: arguments.into(),
        }
    }
}

impl TypedMessage for InvokePromptMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_mcp::InvokePromptMessage");
}

impl MessageTopic for InvokePromptMessage {
    fn topic() -> &'static str {
        TOPIC_MCP_INVOKE_PROMPT
    }
}

impl SharedMessage for InvokePromptMessage {
    fn topic(&self) -> &'static str {
        TOPIC_MCP_INVOKE_PROMPT
    }
}

/// A single message within a resolved prompt.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct PromptMessage {
    /// Role of the message: "system", "user", or "assistant".
    pub role: stabby::string::String,
    /// Content of the message.
    pub content: stabby::string::String,
}

impl PromptMessage {
    /// Create a new prompt message.
    pub fn new(role: &str, content: &str) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
        }
    }
}
