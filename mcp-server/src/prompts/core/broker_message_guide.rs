use crate::prompts::creator::PromptDefinitionCreator;
use crate::prompts::creator::static_prompt_handler;
use crate::prompts::definition::PromptHandler;
use crate::prompts::schema::PromptArgumentsSchema;

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
        PromptArgumentsSchema::empty().to_value()
    }
    fn prompt_handler() -> PromptHandler {
        static_prompt_handler(include_str!("../../../data/prompts/broker_message_guide.md"))
    }
}
