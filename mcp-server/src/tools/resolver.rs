use crate::McpError;
use crate::jsonrpc::JSONRPC_METHOD_NOT_FOUND;
use crate::jsonrpc::JsonRpcResponse;
use crate::tools::ToolDefinition;
use crate::tools::ToolInvocation;
use crate::tools::ToolResultPayload;
use rust_mcp_sdk::schema::ToolInputSchema;
use serde_json::Value;
use std::collections::BTreeMap;

/// Resolver that bridges tool definitions with the rust-mcp-sdk schema types.
/// Provides SDK-facing operations for invoking tools and converting schemas.
pub struct ToolResolver<'a> {
    tools: &'a [ToolDefinition],
}

impl<'a> ToolResolver<'a> {
    /// Create a new resolver over a slice of tool definitions.
    pub fn new(tools: &'a [ToolDefinition]) -> Self {
        Self { tools }
    }

    /// Convert a serde_json::Value JSON schema to the SDK's ToolInputSchema.
    /// The input schema must be a JSON object with "properties" and "required" fields.
    /// Properties are converted from serde_json::Map to BTreeMap<String, serde_json::Map>.
    pub(crate) fn json_schema_to_tool_input_schema(schema: &Value) -> ToolInputSchema {
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
                .collect::<BTreeMap<String, serde_json::Map<String, Value>>>()
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
    /// ServerHandler. Returns Ok(text) on success or Err(McpError) on failure.
    pub async fn invoke_sdk(&self, invocation: ToolInvocation<'_>, name: &str) -> Result<String, McpError> {
        let Some(tool) = self.tools.iter().find(|t| t.name == name) else {
            return Err(McpError::ToolNotFound(name.to_string()));
        };
        match (tool.handler)(invocation).await {
            Ok(result) => Ok(result.to_string()),
            Err(e) => Err(e),
        }
    }

    /// Invoke a tool by name and return a JSON-RPC response.
    pub async fn invoke(&self, id: Option<Value>, invocation: ToolInvocation<'_>, name: &str) -> JsonRpcResponse {
        let Some(tool) = self.tools.iter().find(|t| t.name == name) else {
            return JsonRpcResponse::error(id, JSONRPC_METHOD_NOT_FOUND, format!("Tool {} not found", name), None);
        };

        match (tool.handler)(invocation).await {
            Ok(result) => JsonRpcResponse::success(id, serde_json::to_value(ToolResultPayload::success(result.to_string())).unwrap_or_default()),
            Err(e) => JsonRpcResponse::success(id, serde_json::to_value(ToolResultPayload::error(e.to_string())).unwrap_or_default()),
        }
    }
}
