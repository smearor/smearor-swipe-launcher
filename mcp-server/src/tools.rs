//! MCP tool definitions and invocation helpers.

use crate::McpCommand;
use crate::jsonrpc::JSONRPC_METHOD_NOT_FOUND;
use crate::jsonrpc::JsonRpcResponse;
use crate::jsonrpc::get_object_param;
use crate::jsonrpc::get_optional_string_param;
use crate::jsonrpc::get_string_param;
use async_channel::Sender;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::oneshot;

/// Result type returned by a tool handler.
pub type ToolResult = Result<Value, String>;

/// Future returned by a tool handler.
pub type ToolFuture = Pin<Box<dyn Future<Output = ToolResult> + Send>>;

/// Tool handler signature.
pub type ToolHandler = Box<dyn Fn(Sender<McpCommand>, Option<&Value>) -> ToolFuture + Send + Sync>;

/// Built-in tool definitions exposed by the MCP server.
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub handler: ToolHandler,
}

/// Build the list of core tools available from the MVP.
pub fn core_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "open_area".to_string(),
            description: "Opens a Smearor area by its configured ID.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "area_id": { "type": "string", "description": "Unique area identifier from config.toml" }
                },
                "required": ["area_id"]
            }),
            handler: Box::new(|sender, params| {
                let Some(area_id) = get_string_param(params, "area_id") else {
                    return Box::pin(async move { Err("Missing area_id".to_string()) }) as ToolFuture;
                };
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::OpenArea {
                            area_id,
                            response: oneshot::channel().0,
                        },
                    )
                    .await
                })
            }),
        },
        ToolDefinition {
            name: "close_area".to_string(),
            description: "Closes a currently visible Smearor area.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "area_id": { "type": "string", "description": "Unique area identifier from config.toml" }
                },
                "required": ["area_id"]
            }),
            handler: Box::new(|sender, params| {
                let Some(area_id) = get_string_param(params, "area_id") else {
                    return Box::pin(async move { Err("Missing area_id".to_string()) }) as ToolFuture;
                };
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::CloseArea {
                            area_id,
                            response: oneshot::channel().0,
                        },
                    )
                    .await
                })
            }),
        },
        ToolDefinition {
            name: "list_areas".to_string(),
            description: "Lists all configured Smearor areas with their current visibility and position.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
            handler: Box::new(|sender, _params| {
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::ListAreas {
                            response: oneshot::channel().0,
                        },
                    )
                    .await
                })
            }),
        },
        ToolDefinition {
            name: "focus_area".to_string(),
            description: "Focuses a Smearor area for keyboard navigation.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "area_id": { "type": "string", "description": "Unique area identifier from config.toml" }
                },
                "required": ["area_id"]
            }),
            handler: Box::new(|sender, params| {
                let Some(area_id) = get_string_param(params, "area_id") else {
                    return Box::pin(async move { Err("Missing area_id".to_string()) }) as ToolFuture;
                };
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::FocusArea {
                            area_id,
                            response: oneshot::channel().0,
                        },
                    )
                    .await
                })
            }),
        },
        ToolDefinition {
            name: "send_message".to_string(),
            description: "Publishes a message to a topic on the central message broker.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "topic": { "type": "string", "description": "Broker topic name" },
                    "payload": { "type": "object", "description": "JSON payload to publish" },
                    "target_instance_id": { "type": "string", "description": "Optional target widget/service instance ID" }
                },
                "required": ["topic", "payload"]
            }),
            handler: Box::new(|sender, params| {
                let Some(topic) = get_string_param(params, "topic") else {
                    return Box::pin(async move { Err("Missing topic".to_string()) }) as ToolFuture;
                };
                let Some(payload) = get_object_param(params, "payload") else {
                    return Box::pin(async move { Err("Missing payload".to_string()) }) as ToolFuture;
                };
                let target_instance_id = get_optional_string_param(params, "target_instance_id");
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::SendMessage {
                            topic,
                            payload,
                            target_instance_id,
                            response: oneshot::channel().0,
                        },
                    )
                    .await
                })
            }),
        },
        ToolDefinition {
            name: "toggle_area".to_string(),
            description: "Toggles the visibility of a Smearor area.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "area_id": { "type": "string", "description": "Unique area identifier from config.toml" }
                },
                "required": ["area_id"]
            }),
            handler: Box::new(|sender, params| {
                let Some(area_id) = get_string_param(params, "area_id") else {
                    return Box::pin(async move { Err("Missing area_id".to_string()) }) as ToolFuture;
                };
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::ToggleArea {
                            area_id,
                            response: oneshot::channel().0,
                        },
                    )
                    .await
                })
            }),
        },
        ToolDefinition {
            name: "get_area_config".to_string(),
            description: "Returns the configuration of a Smearor area as JSON.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "area_id": { "type": "string", "description": "Unique area identifier from config.toml" }
                },
                "required": ["area_id"]
            }),
            handler: Box::new(|sender, params| {
                let Some(area_id) = get_string_param(params, "area_id") else {
                    return Box::pin(async move { Err("Missing area_id".to_string()) }) as ToolFuture;
                };
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::GetAreaConfig {
                            area_id,
                            response: oneshot::channel().0,
                        },
                    )
                    .await
                })
            }),
        },
    ]
}

/// Send a command and wait for the launcher core to respond.
async fn send_command_and_wait(sender: Sender<McpCommand>, command: McpCommand) -> ToolResult {
    let (response_tx, response_rx) = oneshot::channel::<Result<String, String>>();
    let command = match command {
        McpCommand::OpenArea { area_id, response: _ } => McpCommand::OpenArea {
            area_id,
            response: response_tx,
        },
        McpCommand::CloseArea { area_id, response: _ } => McpCommand::CloseArea {
            area_id,
            response: response_tx,
        },
        McpCommand::ListAreas { response: _ } => McpCommand::ListAreas { response: response_tx },
        McpCommand::FocusArea { area_id, response: _ } => McpCommand::FocusArea {
            area_id,
            response: response_tx,
        },
        McpCommand::SendMessage {
            topic,
            payload,
            target_instance_id,
            response: _,
        } => McpCommand::SendMessage {
            topic,
            payload,
            target_instance_id,
            response: response_tx,
        },
        McpCommand::ReadResource { uri, response: _ } => McpCommand::ReadResource { uri, response: response_tx },
        McpCommand::ToggleArea { area_id, response: _ } => McpCommand::ToggleArea {
            area_id,
            response: response_tx,
        },
        McpCommand::GetAreaConfig { area_id, response: _ } => McpCommand::GetAreaConfig {
            area_id,
            response: response_tx,
        },
        McpCommand::InvokePluginTool { .. } | McpCommand::InvokePluginResource { .. } => {
            return Err("Plugin invocation commands are handled by the message handler".to_string());
        }
    };

    sender
        .try_send(command)
        .map_err(|e| format!("Failed to send command to launcher core: {}", e))?;

    match tokio::time::timeout(tokio::time::Duration::from_secs(10), response_rx).await {
        Ok(Ok(Ok(result))) => Ok(Value::String(result)),
        Ok(Ok(Err(e))) => Err(e),
        Ok(Err(_)) => Err("Launcher core dropped the response channel".to_string()),
        Err(_) => Err("Tool invocation timed out".to_string()),
    }
}

/// Invoke a core tool by name and return the result as a string for the SDK
/// ServerHandler. Returns Ok(text) on success or Err(message) on failure.
pub async fn invoke_tool_sdk(tools: &[ToolDefinition], sender: Sender<McpCommand>, name: &str, params: Option<&Value>) -> Result<String, String> {
    let Some(tool) = tools.iter().find(|t| t.name == name) else {
        return Err(format!("Tool {} not found", name));
    };
    match (tool.handler)(sender, params).await {
        Ok(result) => Ok(result.to_string()),
        Err(message) => Err(message),
    }
}

/// Invoke a tool by name and return a JSON-RPC response.
pub async fn invoke_tool(tools: &[ToolDefinition], sender: Sender<McpCommand>, id: Option<Value>, name: &str, params: Option<&Value>) -> JsonRpcResponse {
    let Some(tool) = tools.iter().find(|t| t.name == name) else {
        return JsonRpcResponse::error(id, JSONRPC_METHOD_NOT_FOUND, format!("Tool {} not found", name), None);
    };

    match (tool.handler)(sender, params).await {
        Ok(result) => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "content": [{ "type": "text", "text": result.to_string() }],
                "isError": false
            }),
        ),
        Err(message) => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "content": [{ "type": "text", "text": message }],
                "isError": true
            }),
        ),
    }
}
