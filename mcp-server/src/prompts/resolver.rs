use rust_mcp_sdk::schema::ContentBlock;
use rust_mcp_sdk::schema::GetPromptResult;
use rust_mcp_sdk::schema::PromptArgument;
use rust_mcp_sdk::schema::PromptMessage;
use rust_mcp_sdk::schema::Role;
use rust_mcp_sdk::schema::TextContent;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

use crate::McpError;
use crate::prompts::PromptDefinition;

/// Intermediate struct for deserializing a schemars-generated JSON schema
/// into the fields needed for `Vec<PromptArgument>`.
#[derive(Deserialize, Default)]
struct PromptSchemaInput {
    #[serde(default)]
    properties: BTreeMap<String, PromptSchemaProperty>,
    #[serde(default)]
    required: Vec<String>,
}

/// A single property entry within the schema's `properties` map.
#[derive(Deserialize, Default)]
struct PromptSchemaProperty {
    #[serde(default)]
    description: Option<String>,
}

impl From<PromptSchemaInput> for Vec<PromptArgument> {
    fn from(input: PromptSchemaInput) -> Self {
        input
            .properties
            .into_iter()
            .map(|(name, prop)| {
                let is_required = input.required.iter().any(|r| r == &name);
                PromptArgument {
                    description: prop.description,
                    name,
                    required: if is_required { Some(true) } else { None },
                    title: None,
                }
            })
            .collect()
    }
}

/// Resolver that bridges prompt definitions with the rust-mcp-sdk schema types.
/// Provides SDK-facing operations for resolving prompts and converting schemas.
pub struct PromptResolver<'a> {
    prompts: &'a [PromptDefinition],
}

impl<'a> PromptResolver<'a> {
    /// Create a new resolver over a slice of prompt definitions.
    pub fn new(prompts: &'a [PromptDefinition]) -> Self {
        Self { prompts }
    }

    /// Convert a serde_json::Value arguments schema to SDK PromptArgument list.
    /// The input schema is deserialized via `PromptSchemaInput` and mapped
    /// through the `From` trait.
    pub fn schema_to_prompt_arguments(schema: &Value) -> Vec<PromptArgument> {
        let input: PromptSchemaInput = serde_json::from_value(schema.clone()).unwrap_or_default();
        input.into()
    }

    /// Resolve a core prompt by name and return a GetPromptResult for the SDK.
    pub fn get_sdk(&self, name: &str, arguments: &Option<BTreeMap<String, String>>) -> Result<GetPromptResult, McpError> {
        let Some(prompt) = self.prompts.iter().find(|p| p.name == name) else {
            return Err(McpError::PromptNotFound(name.to_string()));
        };
        let content = (prompt.handler)(name, arguments.as_ref())?;
        Ok(GetPromptResult {
            description: Some(prompt.description.clone()),
            messages: vec![PromptMessage {
                content: ContentBlock::TextContent(TextContent::new(content, None, None)),
                role: Role::User,
            }],
            meta: None,
        })
    }
}
