use crate::prompts::creator::PromptDefinitionCreator;
use crate::prompts::creator::static_prompt_handler;
use crate::prompts::definition::PromptHandler;
use crate::prompts::schema::PromptArgumentsSchema;

/// Prompt returning a system message describing the launcher and all available MCP capabilities.
pub struct LauncherOverviewPrompt;

impl PromptDefinitionCreator for LauncherOverviewPrompt {
    fn prompt_name() -> &'static str {
        "launcher_overview"
    }
    fn prompt_description() -> &'static str {
        "Returns a system message describing the launcher and all available MCP capabilities."
    }
    fn prompt_arguments_schema() -> serde_json::Value {
        PromptArgumentsSchema::empty().to_value()
    }
    fn prompt_handler() -> PromptHandler {
        static_prompt_handler(include_str!("../../../data/prompts/launcher_overview.md"))
    }
}
