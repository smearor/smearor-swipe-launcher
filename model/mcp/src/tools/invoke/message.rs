use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_MCP_INVOKE_TOOL: &str = "mcp.invoke.tool";

/// Request sent by the host to a plugin to invoke a registered tool.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct InvokeToolMessage {
    /// Unique tool name as registered by the plugin.
    pub name: stabby::string::String,
    /// Correlation ID used to match the response.
    pub correlation_id: stabby::string::String,
    /// JSON-encoded arguments for the tool invocation.
    pub arguments: stabby::string::String,
}

impl InvokeToolMessage {
    /// Create a new tool invocation request.
    pub fn new(name: &str, correlation_id: &str, arguments: &str) -> Self {
        Self {
            name: name.into(),
            correlation_id: correlation_id.into(),
            arguments: arguments.into(),
        }
    }
}

impl TypedMessage for InvokeToolMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_mcp::InvokeToolMessage");
}

impl MessageTopic for InvokeToolMessage {
    fn topic() -> &'static str {
        TOPIC_MCP_INVOKE_TOOL
    }
}

impl SharedMessage for InvokeToolMessage {
    fn topic(&self) -> &'static str {
        TOPIC_MCP_INVOKE_TOOL
    }
}
