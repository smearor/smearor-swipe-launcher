use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

/// Parameters for reading a plugin resource via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct InvokePluginResourceParams {
    /// The resource URI to read.
    pub uri: String,
    /// The plugin ID that registered the resource.
    pub plugin_id: String,
    /// Correlation ID for matching responses.
    pub correlation_id: String,
}

impl McpCommandVariant for InvokePluginResourceParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::InvokePluginResource(wrapper)
    }
}
