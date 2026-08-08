use crate::prompts::creator::PromptDefinitionCreator;
use crate::prompts::creator::formatted_prompt_handler;
use crate::prompts::definition::PromptHandler;

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
        serde_json::json!({
            "type": "object",
            "properties": {
                "area_id": { "type": "string", "description": "The area to get control instructions for" }
            },
            "required": ["area_id"]
        })
    }
    fn prompt_handler() -> PromptHandler {
        formatted_prompt_handler(|args| {
            let area_id = args.and_then(|a| a.get("area_id").cloned()).unwrap_or_else(|| "<area_id>".to_string());
            format!(
                "To control the area '{area_id}', use the following tools:\n\
                 - open_area: open the area by ID\n\
                 - close_area: close the area\n\
                 - toggle_area: toggle visibility\n\
                 - focus_area: set keyboard focus\n\
                 - get_area_config: retrieve the area's configuration as JSON\n"
            )
        })
    }
}
