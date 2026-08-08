use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

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

impl ToolDefinitionCreator for SendMultipleMessagesParams {
    fn tool_name() -> &'static str {
        "send_multiple_messages"
    }
    fn tool_description() -> &'static str {
        "Publishes multiple messages to the central message broker in a single call. Automatically filters out duplicate messages (same topic + payload). Use this when you need to trigger multiple button actions at once, e.g. turning off all lights in a room."
    }
}
