use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

/// Parameters for listing all managed areas via the command channel.
#[derive(JsonSchema, Deserialize, Default, TypedBuilder)]
pub struct ListAreasParams {}

impl McpCommandVariant for ListAreasParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::ListAreas(wrapper)
    }
}

impl ToolDefinitionCreator for ListAreasParams {
    fn tool_name() -> &'static str {
        "list_areas"
    }
    fn tool_description() -> &'static str {
        "Lists all currently visible/open Smearor launcher areas (Bereiche) with their area_id, visibility state, and position. Use when the user asks for visible areas, 'Bereiche', or 'Areas' in the launcher."
    }
}
