//! MCP prompt definitions and resolution helpers.

mod core;
mod creator;
mod definition;
mod into_sdk_prompt;
mod registered;

use rust_mcp_sdk::schema::ContentBlock;
use rust_mcp_sdk::schema::GetPromptResult;
use rust_mcp_sdk::schema::PromptArgument;
use rust_mcp_sdk::schema::PromptMessage;
use rust_mcp_sdk::schema::Role;
use rust_mcp_sdk::schema::TextContent;
use std::collections::BTreeMap;

pub use definition::PromptDefinition;
pub use definition::PromptHandler;
pub use into_sdk_prompt::IntoSdkPrompt;
pub use into_sdk_prompt::SdkPromptFields;

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
pub fn get_prompt_sdk(prompts: &[PromptDefinition], name: &str, arguments: &Option<BTreeMap<String, String>>) -> Result<GetPromptResult, String> {
    let Some(prompt) = prompts.iter().find(|p| p.name == name) else {
        return Err(format!("Prompt {name} not found"));
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
