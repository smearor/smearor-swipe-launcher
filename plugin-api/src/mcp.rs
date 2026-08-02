use crate::messages::MessageBroadcaster;

/// Trait for plugins and services that register MCP capabilities (tools, resources, prompts).
///
/// Implementations should broadcast `RegisterToolMessage`, `RegisterResourceMessage`,
/// and `RegisterPromptMessage` messages to the message broker during initialization.
pub trait McpCapabilitiesRegistrator: MessageBroadcaster {
    /// Registers all MCP tools, resources, and prompts exposed by this plugin or service.
    fn register_mcp_capabilities(&self);
}
