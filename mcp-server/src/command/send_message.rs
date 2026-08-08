use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

/// Parameters for sending a message to a broker topic via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct SendMessageParams {
    /// Broker topic name (e.g. the click_topic from a button config)
    pub topic: String,
    /// JSON payload to publish (e.g. the click_payload from a button config)
    pub payload: serde_json::Value,
    /// Optional target widget/service instance ID
    #[serde(default)]
    #[builder(default, setter(into))]
    pub target_instance_id: Option<String>,
}

impl McpCommandVariant for SendMessageParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::SendMessage(wrapper)
    }
}
