use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

/// Parameters for getting the web server status via the command channel.
#[derive(JsonSchema, Deserialize, Default, TypedBuilder)]
pub struct WebServerStatusParams {}

impl McpCommandVariant for WebServerStatusParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::WebServerStatus(wrapper)
    }
}

impl ToolDefinitionCreator for WebServerStatusParams {
    fn tool_name() -> &'static str {
        "web_server_status"
    }
    fn tool_description() -> &'static str {
        "Returns the status of the embedded web server, including port, enabled state, and list of active web instances."
    }
}
