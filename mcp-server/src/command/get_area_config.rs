use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

/// Parameters for getting an area's configuration via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct GetAreaConfigParams {
    /// Unique area identifier from config.toml
    pub area_id: String,
}

impl McpCommandVariant for GetAreaConfigParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::GetAreaConfig(wrapper)
    }
}

impl ToolDefinitionCreator for GetAreaConfigParams {
    fn tool_name() -> &'static str {
        "get_area_config"
    }
    fn tool_description() -> &'static str {
        "Returns the full configuration of a Smearor area (Bereich) as JSON, including its buttons and their associated actions. Use this after listing areas to inspect the devices or controls available inside a specific area."
    }
}
