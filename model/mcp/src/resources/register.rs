use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_MCP_REGISTER_RESOURCE: &str = "mcp.register.resource";

/// Message sent by a plugin to register a resource with the MCP server.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct RegisterResourceMessage {
    /// Resource URI exposed to MCP clients.
    pub uri: stabby::string::String,
    /// Display name of the resource.
    pub name: stabby::string::String,
    /// Human-readable description of the resource.
    pub description: stabby::string::String,
    /// MIME type of the resource contents.
    pub mime_type: stabby::string::String,
}

impl RegisterResourceMessage {
    /// Create a new resource registration message.
    pub fn new(uri: &str, name: &str, description: &str, mime_type: &str) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            description: description.into(),
            mime_type: mime_type.into(),
        }
    }
}

impl TypedMessage for RegisterResourceMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_mcp::RegisterResourceMessage");
}

impl MessageTopic for RegisterResourceMessage {
    fn topic() -> &'static str {
        TOPIC_MCP_REGISTER_RESOURCE
    }
}

impl SharedMessage for RegisterResourceMessage {
    fn topic(&self) -> &'static str {
        TOPIC_MCP_REGISTER_RESOURCE
    }
}

/// Description of a resource registered by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
    pub plugin_id: String,
}
