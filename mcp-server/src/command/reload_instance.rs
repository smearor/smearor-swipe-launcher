use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

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
