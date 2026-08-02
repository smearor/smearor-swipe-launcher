use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_MCP_INVOKE_RESOURCE: &str = "mcp.invoke.resource";

/// Request sent by the host to a plugin to read a registered resource.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct InvokeResourceMessage {
    /// Resource URI as registered by the plugin.
    pub uri: stabby::string::String,
    /// Correlation ID used to match the response.
    pub correlation_id: stabby::string::String,
}

impl InvokeResourceMessage {
    /// Create a new resource read request.
    pub fn new(uri: &str, correlation_id: &str) -> Self {
        Self {
            uri: uri.into(),
            correlation_id: correlation_id.into(),
        }
    }
}

impl TypedMessage for InvokeResourceMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_mcp::InvokeResourceMessage");
}

impl MessageTopic for InvokeResourceMessage {
    fn topic() -> &'static str {
        TOPIC_MCP_INVOKE_RESOURCE
    }
}

impl SharedMessage for InvokeResourceMessage {
    fn topic(&self) -> &'static str {
        TOPIC_MCP_INVOKE_RESOURCE
    }
}
