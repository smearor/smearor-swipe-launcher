use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

/// Parameters for starting a launcher instance via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct StartInstanceParams {
    /// Unique identifier of the instance to start
    pub instance_id: String,
}

impl McpCommandVariant for StartInstanceParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::StartInstance(wrapper)
    }
}

impl ToolDefinitionCreator for StartInstanceParams {
    fn tool_name() -> &'static str {
        "launcher_start_instance"
    }
    fn tool_description() -> &'static str {
        "Starts a loaded (Ready) launcher instance by its instance_id. Builds the window or headless areas and transitions the instance to Running state. If the instance is already running, this is a no-op."
    }
}
