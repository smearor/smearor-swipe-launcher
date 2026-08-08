use async_channel::Sender;
use smearor_model_mcp::McpRegistry;
use std::sync::atomic::AtomicU64;

use crate::McpCommand;
use crate::prompts::PromptDefinition;
use crate::resources::ResourceDefinition;
use crate::tools::ToolDefinition;

/// Shared state used by the ServerHandler to process MCP requests.
/// This bridges the SDK's handler trait with the existing McpCommand channel
/// system for communication with the launcher core.
pub struct McpServerState {
    /// Channel used by tool/resource handlers to request actions from the
    /// launcher core.
    pub command_sender: Sender<McpCommand>,
    /// Registered core tools.
    pub tools: Vec<ToolDefinition>,
    /// Registered core resources.
    pub resources: Vec<ResourceDefinition>,
    /// Registered core prompts.
    pub prompts: Vec<PromptDefinition>,
    /// Dynamic registry populated by plugins.
    pub plugin_registry: McpRegistry,
    /// Monotonic counter for MCP invocation correlation IDs.
    pub correlation_counter: AtomicU64,
}
