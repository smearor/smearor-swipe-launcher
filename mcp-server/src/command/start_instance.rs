use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

/// Parameters for starting a launcher instance via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct StartInstanceParams {
    /// Unique identifier of the instance to start
    pub instance_id: String,
}

impl McpCommandVariant for StartInstanceParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::StartInstance(wrapper)
    }
}
