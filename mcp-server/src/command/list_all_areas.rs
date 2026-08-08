use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

/// Parameters for listing all configured areas via the command channel.
#[derive(JsonSchema, Deserialize, Default, TypedBuilder)]
pub struct ListAllAreasParams {}

impl McpCommandVariant for ListAllAreasParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::ListAllAreas(wrapper)
    }
}

impl ToolDefinitionCreator for ListAllAreasParams {
    fn tool_name() -> &'static str {
        "list_all_areas"
    }
    fn tool_description() -> &'static str {
        "Lists every configured Smearor launcher area (Bereiche), including areas that are not currently opened/visible, with their area_id and configuration state. Use when the user asks for a list of all areas, 'alle Bereiche', or 'alle Areas' in the launcher."
    }
}
