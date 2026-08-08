use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

/// Parameters for stopping a launcher instance via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct StopInstanceParams {
    /// Unique identifier of the instance to stop
    pub instance_id: String,
}

impl McpCommandVariant for StopInstanceParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::StopInstance(wrapper)
    }
}

impl ToolDefinitionCreator for StopInstanceParams {
    fn tool_name() -> &'static str {
        "launcher_stop_instance"
    }
    fn tool_description() -> &'static str {
        "Stops a running launcher instance by its instance_id. Closes the window and transitions the instance to Ready state. The instance remains loaded and can be started again. Other instances are not affected."
    }
}
