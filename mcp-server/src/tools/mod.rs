//! MCP tool definitions and invocation helpers.

mod definition;
mod handler;
mod invocation;
mod response;
mod result;

pub use definition::ToolDefinition;
pub use definition::ToolDefinitionCreator;
pub use handler::ToolFuture;
pub use handler::ToolHandler;
pub use invocation::ToolInvocation;
pub use response::ToolContent;
pub use response::ToolResultPayload;
pub use result::ToolResult;

use crate::CloseAreaParams;
use crate::FocusAreaParams;
use crate::GetAreaConfigParams;
use crate::ListAllAreasParams;
use crate::ListAreasParams;
use crate::ListInstancesParams;
use crate::LoadInstanceParams;
use crate::OpenAreaParams;
use crate::OpenTransientAreaParams;
use crate::ReloadInstanceParams;
use crate::SendMessageParams;
use crate::SendMultipleMessagesParams;
use crate::StartInstanceParams;
use crate::StopInstanceParams;
use crate::ToggleAreaParams;
use crate::UnloadInstanceParams;
use crate::WebServerStatusParams;
use crate::jsonrpc::JSONRPC_METHOD_NOT_FOUND;
use crate::jsonrpc::JsonRpcResponse;
use serde_json::Value;

/// Build the list of core tools available from the MVP.
pub fn core_tools() -> Vec<ToolDefinition> {
    vec![
        OpenAreaParams::create_tool_definition(),
        CloseAreaParams::create_tool_definition(),
        ListAreasParams::create_tool_definition(),
        OpenTransientAreaParams::create_tool_definition(),
        FocusAreaParams::create_tool_definition(),
        SendMessageParams::create_tool_definition(),
        SendMultipleMessagesParams::create_tool_definition(),
        ToggleAreaParams::create_tool_definition(),
        ListAllAreasParams::create_tool_definition(),
        GetAreaConfigParams::create_tool_definition(),
        LoadInstanceParams::create_tool_definition(),
        StartInstanceParams::create_tool_definition(),
        StopInstanceParams::create_tool_definition(),
        UnloadInstanceParams::create_tool_definition(),
        ReloadInstanceParams::create_tool_definition(),
        ListInstancesParams::create_tool_definition(),
        WebServerStatusParams::create_tool_definition(),
    ]
}

/// Invoke a core tool by name and return the result as a string for the SDK
/// ServerHandler. Returns Ok(text) on success or Err(message) on failure.
pub async fn invoke_tool_sdk(tools: &[ToolDefinition], invocation: ToolInvocation<'_>, name: &str) -> Result<String, String> {
    let Some(tool) = tools.iter().find(|t| t.name == name) else {
        return Err(format!("Tool {} not found", name));
    };
    match (tool.handler)(invocation).await {
        Ok(result) => Ok(result.to_string()),
        Err(message) => Err(message),
    }
}

/// Invoke a tool by name and return a JSON-RPC response.
pub async fn invoke_tool(tools: &[ToolDefinition], invocation: ToolInvocation<'_>, id: Option<Value>, name: &str) -> JsonRpcResponse {
    let Some(tool) = tools.iter().find(|t| t.name == name) else {
        return JsonRpcResponse::error(id, JSONRPC_METHOD_NOT_FOUND, format!("Tool {} not found", name), None);
    };

    match (tool.handler)(invocation).await {
        Ok(result) => JsonRpcResponse::success(id, serde_json::to_value(ToolResultPayload::success(result.to_string())).unwrap_or_default()),
        Err(message) => JsonRpcResponse::success(id, serde_json::to_value(ToolResultPayload::error(message)).unwrap_or_default()),
    }
}
