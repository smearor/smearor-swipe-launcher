use crate::prompts::creator::PromptDefinitionCreator;
use crate::prompts::creator::static_prompt_handler;
use crate::prompts::definition::PromptHandler;

/// Prompt returning a guide for using send_message with the central message broker.
pub struct BrokerMessageGuidePrompt;

impl PromptDefinitionCreator for BrokerMessageGuidePrompt {
    fn prompt_name() -> &'static str {
        "broker_message_guide"
    }
    fn prompt_description() -> &'static str {
        "Returns a guide for using send_message with the central message broker."
    }
    fn prompt_arguments_schema() -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }
    fn prompt_handler() -> PromptHandler {
        static_prompt_handler(
            "The central message broker allows publishing JSON payloads to named topics.\n\
                   Use the 'send_message' tool with parameters:\n\
                   - topic: the broker topic name (string)\n\
                   - payload: a JSON object to publish\n\
                   - target_instance_id: optional target widget/service instance ID\n\
                   Widgets and services subscribe to topics and react to incoming messages.",
        )
    }
}
