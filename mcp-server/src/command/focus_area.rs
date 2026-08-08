use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

/// Parameters for focusing an area via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct FocusAreaParams {
    /// Unique area identifier from config.toml
    pub area_id: String,
}

impl McpCommandVariant for FocusAreaParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::FocusArea(wrapper)
    }
}
