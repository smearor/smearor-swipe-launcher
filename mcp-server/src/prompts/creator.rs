use crate::prompts::definition::PromptDefinition;
use crate::prompts::definition::PromptHandler;

/// Trait that lets a prompt type generate its own `PromptDefinition`.
pub trait PromptDefinitionCreator {
    /// The MCP prompt name.
    fn prompt_name() -> &'static str;
    /// The human-readable description shown to the LLM.
    fn prompt_description() -> &'static str;
    /// The JSON schema describing the prompt's arguments.
    fn prompt_arguments_schema() -> serde_json::Value;

    /// Create the full `PromptDefinition` with name, description, schema and handler.
    fn create_prompt_definition() -> PromptDefinition {
        PromptDefinition::builder()
            .name(Self::prompt_name())
            .description(Self::prompt_description())
            .arguments_schema(Self::prompt_arguments_schema())
            .handler(Self::prompt_handler())
            .build()
    }

    /// Build the prompt handler. Default implementation returns an error.
    fn prompt_handler() -> PromptHandler {
        Box::new(|_name, _args| Err(format!("Prompt {} has no handler", Self::prompt_name())))
    }
}

/// Helper to create a simple prompt handler that returns a static string.
pub fn static_prompt_handler(content: &'static str) -> PromptHandler {
    Box::new(move |_name, _args| Ok(content.to_string()))
}

/// Helper to create a prompt handler from a static template loaded via `include_str!`.
/// `{key}` placeholders in the template are replaced with the corresponding argument values.
pub fn template_prompt_handler(template: &'static str) -> PromptHandler {
    Box::new(move |_name, args| {
        let mut result = template.to_string();
        if let Some(args) = args {
            for (key, value) in args {
                result = result.replace(&format!("{{{key}}}"), value);
            }
        }
        Ok(result)
    })
}
