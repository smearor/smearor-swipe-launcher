use crate::prompts::creator::PromptDefinitionCreator;
use crate::prompts::creator::static_prompt_handler;
use crate::prompts::definition::PromptHandler;

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
        serde_json::json!({"type": "object", "properties": {}})
    }
    fn prompt_handler() -> PromptHandler {
        static_prompt_handler(
            "You are interacting with the Smearor Swipe Launcher. Use 'list_all_areas' to discover available areas, 'open_area' to open them, and 'send_message' to communicate with widgets and services. Resources and tools are dynamically registered by plugins.",
        )
    }
}
