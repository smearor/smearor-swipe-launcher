use rust_mcp_sdk::schema::ContentBlock;
use rust_mcp_sdk::schema::GetPromptResult;
use rust_mcp_sdk::schema::PromptArgument;
use rust_mcp_sdk::schema::PromptMessage;
use rust_mcp_sdk::schema::Role;
use rust_mcp_sdk::schema::TextContent;
use std::collections::BTreeMap;

use crate::McpError;
use crate::prompts::PromptDefinition;

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
    pub fn schema_to_prompt_arguments(schema: &serde_json::Value) -> Vec<PromptArgument> {
        let Some(props) = schema.get("properties").and_then(|p| p.as_object()) else {
            return vec![];
        };
        let required: Vec<String> = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        props
            .iter()
            .map(|(name, value)| {
                let description = value.get("description").and_then(|d| d.as_str()).map(String::from);
                let is_required = required.iter().any(|r| r == name);
                PromptArgument {
                    description,
                    name: name.clone(),
                    required: if is_required { Some(true) } else { None },
                    title: None,
                }
            })
            .collect()
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
