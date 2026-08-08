use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

/// Parameters for getting the web server status via the command channel.
#[derive(JsonSchema, Deserialize, Default, TypedBuilder)]
pub struct WebServerStatusParams {}

impl McpCommandVariant for WebServerStatusParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::WebServerStatus(wrapper)
    }
}
