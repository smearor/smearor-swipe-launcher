use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;

/// A single message in a send_multiple_messages call.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct MessageItem {
    /// Broker topic name
    pub topic: String,
    /// JSON payload to publish
    pub payload: serde_json::Value,
    /// Optional target widget/service instance ID
    #[serde(default)]
    #[builder(default, setter(into))]
    pub target_instance_id: Option<String>,
}

/// Parameters for sending multiple broker messages via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct SendMultipleMessagesParams {
    /// Array of messages to send. Each message has a topic, payload, and optional target_instance_id.
    pub messages: Vec<MessageItem>,
}

impl McpCommandVariant for SendMultipleMessagesParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::SendMultipleMessages(wrapper)
    }
}
