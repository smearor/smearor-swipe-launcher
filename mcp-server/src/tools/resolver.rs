use crate::McpError;
use crate::jsonrpc::JSONRPC_METHOD_NOT_FOUND;
use crate::jsonrpc::JsonRpcResponse;
use crate::tools::ToolDefinition;
use crate::tools::ToolInvocation;
use crate::tools::ToolResultPayload;
use rust_mcp_sdk::schema::ToolInputSchema;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// Intermediate struct for deserializing a schemars-generated JSON schema
/// into the fields needed by `ToolInputSchema`.
#[derive(Deserialize, Default)]
struct ToolSchemaInput {
    #[serde(default)]
    properties: Option<BTreeMap<String, serde_json::Map<String, Value>>>,
    #[serde(default)]
    required: Vec<String>,
    #[serde(rename = "$schema", default)]
    schema: Option<String>,
}

impl From<ToolSchemaInput> for ToolInputSchema {
    fn from(input: ToolSchemaInput) -> Self {
        ToolInputSchema::new(input.required, input.properties, input.schema)
    }
}

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
    /// The input schema is deserialized via `ToolSchemaInput` and mapped
    /// to the SDK's constructor.
    pub(crate) fn json_schema_to_tool_input_schema(schema: &Value) -> ToolInputSchema {
        let input: ToolSchemaInput = serde_json::from_value(schema.clone()).unwrap_or_default();
        input.into()
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
