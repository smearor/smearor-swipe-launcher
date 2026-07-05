//! Shared message types for the Smearor MCP server.
//!
//! Plugins and the launcher host use these messages to register dynamic tools
//! and resources that the MCP server exposes to external AI clients.

use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub mod registry;

pub use registry::McpRegistry;
pub use registry::RegisteredResource;
pub use registry::RegisteredTool;

pub const TOPIC_MCP_REGISTER_TOOL: &str = "mcp.register.tool";
pub const TOPIC_MCP_REGISTER_RESOURCE: &str = "mcp.register.resource";
pub const TOPIC_MCP_INVOKE_TOOL: &str = "mcp.invoke.tool";
pub const TOPIC_MCP_INVOKE_RESOURCE: &str = "mcp.invoke.resource";
pub const TOPIC_MCP_TOOL_RESPONSE: &str = "mcp.tool.response";
pub const TOPIC_MCP_RESOURCE_RESPONSE: &str = "mcp.resource.response";

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
