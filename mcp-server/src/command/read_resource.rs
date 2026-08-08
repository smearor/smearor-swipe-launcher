use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

/// Parameters for reading a core resource by URI via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct ReadResourceParams {
    /// The resource URI to read.
    pub uri: String,
}

impl McpCommandVariant for ReadResourceParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::ReadResource(wrapper)
    }
}
