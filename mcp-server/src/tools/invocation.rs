use crate::McpCommand;
use async_channel::Sender;
use serde_json::Value;

/// Bundles the command sender and optional parameters passed to a tool handler.
pub struct ToolInvocation<'a> {
    /// Channel sender for dispatching commands to the launcher core.
    pub sender: Sender<McpCommand>,
    /// Optional JSON parameters for the tool call.
    pub params: Option<&'a Value>,
}

impl<'a> ToolInvocation<'a> {
    /// Create a new tool invocation context.
    pub fn new(sender: Sender<McpCommand>, params: Option<&'a Value>) -> Self {
        Self { sender, params }
    }
}
