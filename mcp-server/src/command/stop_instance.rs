use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

/// Parameters for stopping a launcher instance via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct StopInstanceParams {
    /// Unique identifier of the instance to stop
    pub instance_id: String,
}

impl McpCommandVariant for StopInstanceParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::StopInstance(wrapper)
    }
}
