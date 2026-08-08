use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

/// Parameters for invoking a plugin tool via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct InvokePluginToolParams {
    /// The tool name to invoke.
    pub name: String,
    /// The plugin ID that registered the tool.
    pub plugin_id: String,
    /// Correlation ID for matching responses.
    pub correlation_id: String,
    /// JSON-encoded arguments for the tool invocation.
    pub arguments: serde_json::Value,
}

impl McpCommandVariant for InvokePluginToolParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::InvokePluginTool(wrapper)
    }
}
