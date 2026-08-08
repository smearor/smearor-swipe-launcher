use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

/// Parameters for listing all running launcher instances via the command channel.
#[derive(JsonSchema, Deserialize, Default, TypedBuilder)]
pub struct ListInstancesParams {}

impl McpCommandVariant for ListInstancesParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::ListInstances(wrapper)
    }
}

impl ToolDefinitionCreator for ListInstancesParams {
    fn tool_name() -> &'static str {
        "launcher_list_instances"
    }
    fn tool_description() -> &'static str {
        "Lists all currently running launcher instances with their IDs, types, and window states."
    }
}
