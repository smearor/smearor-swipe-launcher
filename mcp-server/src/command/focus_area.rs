use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

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

impl ToolDefinitionCreator for FocusAreaParams {
    fn tool_name() -> &'static str {
        "focus_area"
    }
    fn tool_description() -> &'static str {
        "Focuses a Smearor area for keyboard navigation."
    }
}
