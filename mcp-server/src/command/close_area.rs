use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

/// Parameters for closing an area via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct CloseAreaParams {
    /// Unique area identifier from config.toml
    pub area_id: String,
}

impl McpCommandVariant for CloseAreaParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::CloseArea(wrapper)
    }
}
