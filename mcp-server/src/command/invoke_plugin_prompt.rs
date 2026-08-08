use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

/// Parameters for invoking a plugin prompt via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct InvokePluginPromptParams {
    /// The prompt name to invoke.
    pub name: String,
    /// The plugin ID that registered the prompt.
    pub plugin_id: String,
    /// Correlation ID for matching responses.
    pub correlation_id: String,
    /// JSON-encoded arguments for the prompt invocation.
    pub arguments: serde_json::Value,
}

impl McpCommandVariant for InvokePluginPromptParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::InvokePluginPrompt(wrapper)
    }
}
