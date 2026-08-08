//! MCP tool definitions and invocation helpers.

mod creator;
mod definition;
mod handler;
mod into_sdk_tool;
mod invocation;
mod registered;
mod response;
mod result;

pub use creator::ToolDefinitionCreator;
pub use definition::ToolDefinition;
pub use handler::ToolFuture;
pub use handler::ToolHandler;
pub use into_sdk_tool::IntoSdkTool;
pub use into_sdk_tool::SdkToolFields;
pub use invocation::ToolInvocation;
pub use response::ToolContent;
pub use response::ToolResultPayload;
pub use result::ToolResult;

use crate::jsonrpc::JSONRPC_METHOD_NOT_FOUND;
use crate::jsonrpc::JsonRpcResponse;
use rust_mcp_sdk::schema::ToolInputSchema;
use serde_json::Value;

/// Convert a serde_json::Value JSON schema to the SDK's ToolInputSchema.
/// The input schema must be a JSON object with "properties" and "required" fields.
/// Properties are converted from serde_json::Map to BTreeMap<String, serde_json::Map>.
pub(crate) fn json_schema_to_tool_input_schema(schema: &serde_json::Value) -> ToolInputSchema {
    let properties = schema.get("properties").and_then(|p| p.as_object()).map(|map| {
        map.iter()
            .map(|(k, v)| {
                let inner = match v {
                    serde_json::Value::Object(obj) => obj.clone(),
                    _ => {
                        let mut m = serde_json::Map::new();
                        m.insert("value".to_string(), v.clone());
                        m
                    }
                };
                (k.clone(), inner)
            })
            .collect::<std::collections::BTreeMap<String, serde_json::Map<String, serde_json::Value>>>()
    });
    let required = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
        .unwrap_or_default();
    let schema_uri = schema.get("$schema").and_then(|t| t.as_str()).map(String::from);
    ToolInputSchema::new(required, properties, schema_uri)
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
