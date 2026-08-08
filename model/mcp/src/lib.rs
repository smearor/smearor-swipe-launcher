//! Shared message types for the Smearor MCP server.
//!
//! Plugins and the launcher host use these messages to register dynamic tools
//! and resources that the MCP server exposes to external AI clients.

pub mod prompts;
pub mod registry;
pub mod requests;
pub mod resources;
pub mod tools;

pub use prompts::invoke::error::InvokePromptError;
pub use prompts::invoke::error::UnknownPromptError;
pub use prompts::invoke::message::InvokePromptMessage;
pub use prompts::invoke::message::PromptMessage;
pub use prompts::invoke::message::TOPIC_MCP_INVOKE_PROMPT;
pub use prompts::invoke::response::InvokePromptResponse;
pub use prompts::invoke::response::TOPIC_MCP_PROMPT_RESPONSE;
pub use prompts::register::RegisterPromptMessage;
pub use prompts::register::RegisteredPrompt;
pub use prompts::register::TOPIC_MCP_REGISTER_PROMPT;
pub use prompts::template::render_template;
pub use registry::McpRegistry;
pub use requests::ButtonActionArgs;
pub use requests::NoArgs;
pub use resources::invoke::error::InvokeResourceError;
pub use resources::invoke::error::UnknownResourceError;
pub use resources::invoke::message::InvokeResourceMessage;
pub use resources::invoke::message::TOPIC_MCP_INVOKE_RESOURCE;
pub use resources::invoke::response::InvokeResourceResponse;
pub use resources::invoke::response::TOPIC_MCP_RESOURCE_RESPONSE;
pub use resources::register::RegisterResourceMessage;
pub use resources::register::RegisteredResource;
pub use resources::register::TOPIC_MCP_REGISTER_RESOURCE;
pub use tools::invoke::error::InvokeToolError;
pub use tools::invoke::error::UnknownToolError;
pub use tools::invoke::message::InvokeToolMessage;
pub use tools::invoke::message::TOPIC_MCP_INVOKE_TOOL;
pub use tools::invoke::response::InvokeToolResponse;
pub use tools::invoke::response::TOPIC_MCP_TOOL_RESPONSE;
pub use tools::register::RegisterToolMessage;
pub use tools::register::RegisteredTool;
pub use tools::register::TOPIC_MCP_REGISTER_TOOL;
