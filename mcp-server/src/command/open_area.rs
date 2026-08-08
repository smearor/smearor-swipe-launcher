use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

/// Parameters for opening an area via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct OpenAreaParams {
    /// Unique area identifier from config.toml
    pub area_id: String,
}

impl McpCommandVariant for OpenAreaParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::OpenArea(wrapper)
    }
}

impl ToolDefinitionCreator for OpenAreaParams {
    fn tool_name() -> &'static str {
        "open_area"
    }
    fn tool_description() -> &'static str {
        "Opens a Smearor area by its configured ID."
    }
}
