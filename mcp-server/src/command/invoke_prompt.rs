use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::BTreeMap;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

/// Parameters for invoking a prompt via the `invoke_prompt` MCP tool.
///
/// This tool bridges `prompts/get` for MCP clients that only support `tools/call`.
/// It is intercepted directly in `handle_call_tool_request` and delegates to
/// `handle_get_prompt_request`. The `McpCommand` variant exists to satisfy the
/// `ToolDefinitionCreator` trait requirement and is a no-op in the launcher core.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct InvokePromptParams {
    /// The name of the prompt to invoke (e.g. 'weather_query_guide', 'app_launch_guide').
    pub prompt_name: String,
    /// Optional arguments for the prompt invocation.
    #[serde(default)]
    #[builder(default)]
    pub arguments: Option<BTreeMap<String, String>>,
}

impl McpCommandVariant for InvokePromptParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::InvokePrompt(wrapper)
    }
}

impl ToolDefinitionCreator for InvokePromptParams {
    fn tool_name() -> &'static str {
        "invoke_prompt"
    }
    fn tool_description() -> &'static str {
        "Invokes a registered MCP prompt by name and returns the resolved prompt messages as JSON. Use this to retrieve prompt content from plugins and core prompts. Check voice_assistant://prompt_catalog for available prompt names."
    }
}
