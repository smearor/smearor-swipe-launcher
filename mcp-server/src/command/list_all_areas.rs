use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

/// Parameters for listing all configured areas via the command channel.
#[derive(JsonSchema, Deserialize, Default, TypedBuilder)]
pub struct ListAllAreasParams {}

impl McpCommandVariant for ListAllAreasParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::ListAllAreas(wrapper)
    }
}
