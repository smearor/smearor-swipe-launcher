use async_channel::Sender;
use smearor_model_mcp::McpRegistry;
use std::sync::atomic::AtomicU64;
use typed_builder::TypedBuilder;

use crate::McpCommand;
use crate::prompts::PromptDefinition;
use crate::resources::ResourceDefinition;
use crate::tools::ToolDefinition;

/// Shared state used by the ServerHandler to process MCP requests.
/// This bridges the SDK's handler trait with the existing McpCommand channel
/// system for communication with the launcher core.
#[derive(TypedBuilder)]
pub struct McpServerState {
    /// Channel used by tool/resource handlers to request actions from the
    /// launcher core.
    pub command_sender: Sender<McpCommand>,
    /// Registered core tools.
    #[builder(default = ToolDefinition::core_tools())]
    pub tools: Vec<ToolDefinition>,
    /// Registered core resources.
    #[builder(default = ResourceDefinition::core_resources())]
    pub resources: Vec<ResourceDefinition>,
    /// Registered core prompts.
    #[builder(default = PromptDefinition::core_prompts())]
    pub prompts: Vec<PromptDefinition>,
    /// Dynamic registry populated by plugins.
    pub plugin_registry: McpRegistry,
    /// Monotonic counter for MCP invocation correlation IDs.
    #[builder(default = AtomicU64::new(1))]
    pub correlation_counter: AtomicU64,
}
