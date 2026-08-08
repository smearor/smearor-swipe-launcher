use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

/// Parameters for listing all managed areas via the command channel.
#[derive(JsonSchema, Deserialize, Default, TypedBuilder)]
pub struct ListAreasParams {}

impl McpCommandVariant for ListAreasParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::ListAreas(wrapper)
    }
}
