use crate::InvokeResourceError;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_MCP_RESOURCE_RESPONSE: &str = "mcp.resource.response";

/// Response returned by a plugin after reading a registered resource.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct InvokeResourceResponse {
    /// Correlation ID matching the request.
    pub correlation_id: stabby::string::String,
    /// Resource contents. Empty on error.
    pub contents: stabby::string::String,
    /// Error message. Empty when the read succeeded.
    pub error: stabby::string::String,
}

impl InvokeResourceResponse {
    /// Create a successful resource read response.
    pub fn success(correlation_id: &str, contents: &str) -> Self {
        Self {
            correlation_id: correlation_id.into(),
            contents: contents.into(),
            error: "".into(),
        }
    }

    /// Create an error resource read response.
    pub fn error(correlation_id: &str, error: &str) -> Self {
        Self {
            correlation_id: correlation_id.into(),
            contents: "".into(),
            error: error.into(),
        }
    }
}

impl TypedMessage for InvokeResourceResponse {
    const TYPE_ID: u64 = generate_type_id("smearor_model_mcp::InvokeResourceResponse");
}

impl MessageTopic for InvokeResourceResponse {
    fn topic() -> &'static str {
        TOPIC_MCP_RESOURCE_RESPONSE
    }
}

impl SharedMessage for InvokeResourceResponse {
    fn topic(&self) -> &'static str {
        TOPIC_MCP_RESOURCE_RESPONSE
    }
}

impl From<InvokeResourceError> for InvokeResourceResponse {
    fn from(e: InvokeResourceError) -> Self {
        InvokeResourceResponse::error(&e.correlation_id, &format!("Unknown mcp resource {}", e.uri))
    }
}
