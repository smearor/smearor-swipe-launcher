use crate::InvokeToolError;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_MCP_TOOL_RESPONSE: &str = "mcp.tool.response";

/// Response returned by a plugin after invoking a registered tool.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct InvokeToolResponse {
    /// Correlation ID matching the request.
    pub correlation_id: stabby::string::String,
    /// Tool result as a JSON string. Empty on error.
    pub result: stabby::string::String,
    /// Error message. Empty when the invocation succeeded.
    pub error: stabby::string::String,
}

impl InvokeToolResponse {
    /// Create a successful tool invocation response.
    pub fn success(correlation_id: &str, result: &str) -> Self {
        Self {
            correlation_id: correlation_id.into(),
            result: result.into(),
            error: "".into(),
        }
    }

    /// Create an error tool invocation response.
    pub fn error(correlation_id: &str, error: &str) -> Self {
        Self {
            correlation_id: correlation_id.into(),
            result: "".into(),
            error: error.into(),
        }
    }
}

impl TypedMessage for InvokeToolResponse {
    const TYPE_ID: u64 = generate_type_id("smearor_model_mcp::InvokeToolResponse");
}

impl MessageTopic for InvokeToolResponse {
    fn topic() -> &'static str {
        TOPIC_MCP_TOOL_RESPONSE
    }
}

impl SharedMessage for InvokeToolResponse {
    fn topic(&self) -> &'static str {
        TOPIC_MCP_TOOL_RESPONSE
    }
}

impl From<InvokeToolError> for InvokeToolResponse {
    fn from(e: InvokeToolError) -> Self {
        InvokeToolResponse::error(&e.correlation_id, &format!("Unknown mcp tool {}", e.name))
    }
}
