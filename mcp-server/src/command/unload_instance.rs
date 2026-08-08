use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

/// Parameters for unloading a launcher instance via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct UnloadInstanceParams {
    /// Unique identifier of the instance to unload
    pub instance_id: String,
}

impl McpCommandVariant for UnloadInstanceParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::UnloadInstance(wrapper)
    }
}
