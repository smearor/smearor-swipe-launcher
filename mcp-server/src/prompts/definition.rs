use std::collections::BTreeMap;

use typed_builder::TypedBuilder;

use crate::prompts::core::AreaControlHelpPrompt;
use crate::prompts::core::BrokerMessageGuidePrompt;
use crate::prompts::core::LauncherOverviewPrompt;
use crate::prompts::core::ToolShortcutGuidePrompt;
use crate::prompts::creator::PromptDefinitionCreator;
use crate::prompts::into_sdk_prompt::SdkPromptFields;

/// Prompt handler signature.
pub type PromptHandler = Box<dyn Fn(&str, Option<&BTreeMap<String, String>>) -> Result<String, String> + Send + Sync>;

/// Built-in prompt definition exposed by the MCP server.
#[derive(TypedBuilder)]
pub struct PromptDefinition {
    /// A short human-readable name for the prompt.
    #[builder(setter(into))]
    pub name: String,
    /// A human-readable description of what the prompt provides.
    #[builder(setter(into))]
    pub description: String,
    /// The JSON schema describing the prompt's arguments.
    pub arguments_schema: serde_json::Value,
    /// The handler invoked when the prompt is resolved, returning the content as a string.
    pub handler: PromptHandler,
}

impl SdkPromptFields for PromptDefinition {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn arguments_schema(&self) -> &serde_json::Value {
        &self.arguments_schema
    }
}

impl PromptDefinition {
    /// Build the list of core prompts available from the MCP server.
    pub fn core_prompts() -> Vec<PromptDefinition> {
        vec![
            LauncherOverviewPrompt::create_prompt_definition(),
            AreaControlHelpPrompt::create_prompt_definition(),
            BrokerMessageGuidePrompt::create_prompt_definition(),
            ToolShortcutGuidePrompt::create_prompt_definition(),
        ]
    }
}
