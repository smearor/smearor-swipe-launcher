use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_MCP_REGISTER_TOOL: &str = "mcp.register.tool";

/// Message sent by a plugin to register a tool with the MCP server.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct RegisterToolMessage {
    /// Unique tool name exposed to MCP clients.
    pub name: stabby::string::String,
    /// Human-readable description of the tool.
    pub description: stabby::string::String,
    /// JSON schema for the tool's input parameters.
    pub input_schema: stabby::string::String,
}

impl RegisterToolMessage {
    /// Create a new tool registration message.
    pub fn new(name: &str, description: &str, input_schema: &str) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: input_schema.into(),
        }
    }
}

impl TypedMessage for RegisterToolMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_mcp::RegisterToolMessage");
}

impl MessageTopic for RegisterToolMessage {
    fn topic() -> &'static str {
        TOPIC_MCP_REGISTER_TOOL
    }
}

impl SharedMessage for RegisterToolMessage {
    fn topic(&self) -> &'static str {
        TOPIC_MCP_REGISTER_TOOL
    }
}

/// Description of a tool registered by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub plugin_id: String,
}
