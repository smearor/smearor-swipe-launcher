use rust_mcp_sdk::schema::Prompt;

use crate::prompts::PromptResolver;

/// Trait for converting prompt-like types into the SDK `Prompt` type.
pub trait IntoSdkPrompt {
    /// Convert into the SDK `Prompt` representation.
    fn into_sdk_prompt(&self) -> Prompt;
}

/// Fields shared by all prompt-like types that can be converted to an SDK `Prompt`.
pub trait SdkPromptFields {
    /// A short human-readable name for the prompt.
    fn name(&self) -> &str;
    /// A human-readable description of what the prompt provides.
    fn description(&self) -> &str;
    /// The JSON schema describing the prompt's arguments.
    fn arguments_schema(&self) -> &serde_json::Value;
}

impl<T: SdkPromptFields> IntoSdkPrompt for T {
    fn into_sdk_prompt(&self) -> Prompt {
        Prompt {
            name: self.name().to_string(),
            description: Some(self.description().to_string()),
            arguments: PromptResolver::schema_to_prompt_arguments(self.arguments_schema()),
            icons: vec![],
            meta: None,
            title: None,
        }
    }
}
