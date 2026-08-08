use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

/// Parameters for opening a transient area overlay via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct OpenTransientAreaParams {
    /// The area ID to open as a transient overlay.
    pub area_id: String,
    /// The source area ID to use as the overlay source, if specified.
    #[serde(default)]
    #[builder(default, setter(into))]
    pub source_area_id: Option<String>,
}

impl McpCommandVariant for OpenTransientAreaParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::OpenTransientArea(wrapper)
    }
}
