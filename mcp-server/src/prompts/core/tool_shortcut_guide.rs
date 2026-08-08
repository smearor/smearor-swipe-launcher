use crate::prompts::creator::PromptDefinitionCreator;
use crate::prompts::creator::static_prompt_handler;
use crate::prompts::definition::PromptHandler;
use crate::prompts::schema::PromptArgumentsSchema;

/// Prompt returning a shortcut map for common user requests to avoid unnecessary tool discovery.
pub struct ToolShortcutGuidePrompt;

impl PromptDefinitionCreator for ToolShortcutGuidePrompt {
    fn prompt_name() -> &'static str {
        "tool_shortcut_guide"
    }
    fn prompt_description() -> &'static str {
        "Returns a shortcut map for common user requests to avoid unnecessary tool discovery."
    }
    fn prompt_arguments_schema() -> serde_json::Value {
        PromptArgumentsSchema::empty().to_value()
    }
    fn prompt_handler() -> PromptHandler {
        static_prompt_handler(include_str!("../../../data/prompts/tool_shortcut_guide.md"))
    }
}
