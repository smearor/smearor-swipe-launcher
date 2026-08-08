use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

/// Parameters for toggling an area's visibility via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct ToggleAreaParams {
    /// Unique area identifier from config.toml
    pub area_id: String,
}

impl McpCommandVariant for ToggleAreaParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::ToggleArea(wrapper)
    }
}

impl ToolDefinitionCreator for ToggleAreaParams {
    fn tool_name() -> &'static str {
        "toggle_area"
    }
    fn tool_description() -> &'static str {
        "Toggles the visibility of a Smearor area."
    }
}
