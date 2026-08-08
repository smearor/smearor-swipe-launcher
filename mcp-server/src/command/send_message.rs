use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

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

impl ToolDefinitionCreator for SendMessageParams {
    fn tool_name() -> &'static str {
        "send_message"
    }
    fn tool_description() -> &'static str {
        "Publishes a message to a topic on the central message broker. Use this to trigger button actions: after reading an area config, use the button's click_topic as the topic and click_payload as the payload to activate devices like lights."
    }
}
