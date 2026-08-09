use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

/// Parameters for the `read_resource` MCP tool.
///
/// This tool bridges `resources/read` for MCP clients that only support `tools/call`.
/// It is intercepted directly in `handle_call_tool_request` and delegates to
/// `handle_read_resource_request`. The `McpCommand` variant exists to satisfy the
/// `ToolDefinitionCreator` trait requirement and is a no-op in the launcher core.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct ReadResourceToolParams {
    /// The resource URI to read (e.g. 'hyprland://workspace-snapshot', 'hyprland://state').
    pub uri: String,
}

impl McpCommandVariant for ReadResourceToolParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::ReadResourceTool(wrapper)
    }
}

impl ToolDefinitionCreator for ReadResourceToolParams {
    fn tool_name() -> &'static str {
        "read_resource"
    }
    fn tool_description() -> &'static str {
        "Reads a resource by URI and returns its contents. Use this to query state from plugins, e.g. 'hyprland://workspace-snapshot' for workspace info, 'hyprland://state' for compositor state, 'hyprland://active-window' for the focused window."
    }
}
