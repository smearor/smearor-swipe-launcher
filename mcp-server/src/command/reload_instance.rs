use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

/// Parameters for hot-reloading a launcher instance via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct ReloadInstanceParams {
    /// Unique identifier of the instance to reload
    pub instance_id: String,
    /// Optional path to a new config file. If omitted, the original config path is reused.
    #[serde(default)]
    #[builder(default, setter(into))]
    pub config_path: Option<String>,
}

impl McpCommandVariant for ReloadInstanceParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::ReloadInstance(wrapper)
    }
}

impl ToolDefinitionCreator for ReloadInstanceParams {
    fn tool_name() -> &'static str {
        "launcher_reload_instance"
    }
    fn tool_description() -> &'static str {
        "Hot-reloads a launcher instance by its instance_id. Stops the instance if running, unloads it, re-loads from its config file, and restores the previous lifecycle state (Running or Ready)."
    }
}
