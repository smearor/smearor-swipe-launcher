use crate::prompts::creator::PromptDefinitionCreator;
use crate::prompts::creator::template_prompt_handler;
use crate::prompts::definition::PromptHandler;
use crate::prompts::schema::PromptArgumentsSchema;

/// Prompt returning instructions for controlling a specific area.
pub struct AreaControlHelpPrompt;

impl PromptDefinitionCreator for AreaControlHelpPrompt {
    fn prompt_name() -> &'static str {
        "area_control_help"
    }
    fn prompt_description() -> &'static str {
        "Returns instructions for controlling a specific area."
    }
    fn prompt_arguments_schema() -> serde_json::Value {
        PromptArgumentsSchema::empty()
            .property("area_id", "string", "The area to get control instructions for")
            .required("area_id")
            .to_value()
    }
    fn prompt_handler() -> PromptHandler {
        template_prompt_handler(include_str!("../../../data/prompts/area_control_help.md"))
    }
}
