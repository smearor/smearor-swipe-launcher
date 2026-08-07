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
            description: "Lists all currently visible/open Smearor launcher areas (Bereiche) with their area_id, visibility state, and position. Use when the user asks for visible areas, 'Bereiche', or 'Areas' in the launcher.".to_string(),
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
            name: "open_transient_area".to_string(),
            description: "Opens a Smearor area as a transient overlay on top of a source area (simulates a button click).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "area_id": { "type": "string", "description": "Unique area identifier from config.toml" },
                    "source_area_id": { "type": "string", "description": "ID of the managed area to use as source for the overlay. Defaults to the first scroll area." }
                },
                "required": ["area_id"]
            }),
            handler: Box::new(|sender, params| {
                let Some(area_id) = get_string_param(params, "area_id") else {
                    return Box::pin(async move { Err("Missing area_id".to_string()) }) as ToolFuture;
                };
                let source_area_id = get_optional_string_param(params, "source_area_id");
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::OpenTransientArea {
                            area_id,
                            source_area_id,
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
            description: "Publishes a message to a topic on the central message broker. Use this to trigger button actions: after reading an area config, use the button's click_topic as the topic and click_payload as the payload to activate devices like lights.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "topic": { "type": "string", "description": "Broker topic name (e.g. the click_topic from a button config)" },
                    "payload": { "type": "object", "description": "JSON payload to publish (e.g. the click_payload from a button config)" },
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
            name: "send_multiple_messages".to_string(),
            description: "Publishes multiple messages to the central message broker in a single call. Automatically filters out duplicate messages (same topic + payload). Use this when you need to trigger multiple button actions at once, e.g. turning off all lights in a room.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "messages": {
                        "type": "array",
                        "description": "Array of messages to send. Each message has a topic, payload, and optional target_instance_id.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "topic": { "type": "string", "description": "Broker topic name" },
                                "payload": { "type": "object", "description": "JSON payload to publish" },
                                "target_instance_id": { "type": "string", "description": "Optional target widget/service instance ID" }
                            },
                            "required": ["topic", "payload"]
                        }
                    }
                },
                "required": ["messages"]
            }),
            handler: Box::new(|sender, params| {
                let Some(messages_array) = params.and_then(|p| p.get("messages")).and_then(|v| v.as_array()) else {
                    return Box::pin(async move { Err("Missing 'messages' array. Each message must be an object with 'topic' (string) and 'payload' (object). Example: {\"messages\": [{\"topic\": \"service.http.request\", \"payload\": {\"method\": \"Get\", \"url\": \"...\", \"response_topic\": \"...\"}}]}".to_string()) }) as ToolFuture;
                };
                let mut messages = Vec::new();
                let mut errors: Vec<String> = Vec::new();
                for (index, item) in messages_array.iter().enumerate() {
                    if !item.is_object() {
                        errors.push(format!("Message {}: not a JSON object (got {}). Each message must be an object with 'topic' and 'payload' fields.", index, item));
                        continue;
                    }
                    let Some(topic) = item.get("topic").and_then(|v| v.as_str()).map(|s| s.to_string()) else {
                        errors.push(format!("Message {}: missing or non-string 'topic' field. The 'topic' must be a string, e.g. \"service.http.request\".", index));
                        continue;
                    };
                    let Some(payload) = item.get("payload").cloned() else {
                        errors.push(format!("Message {}: missing 'payload' field. The 'payload' must be a JSON object, e.g. {{\"method\": \"Get\", \"url\": \"...\", \"response_topic\": \"...\"}}.", index));
                        continue;
                    };
                    if !payload.is_object() {
                        errors.push(format!("Message {}: 'payload' is not a JSON object (got {}). It must be an object.", index, payload));
                        continue;
                    }
                    let target_instance_id = item.get("target_instance_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    messages.push((topic, payload, target_instance_id));
                }
                if !errors.is_empty() {
                    let error_detail = errors.join("; ");
                    return Box::pin(async move { Err(format!("Invalid messages format: {error_detail}. Fix the format and retry. Each message must be: {{\"topic\": \"<string>\", \"payload\": {{<object>}}, \"target_instance_id\": \"<optional string>\"}}")) }) as ToolFuture;
                }
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::SendMultipleMessages {
                            messages,
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
            name: "list_all_areas".to_string(),
            description: "Lists every configured Smearor launcher area (Bereiche), including areas that are not currently opened/visible, with their area_id and configuration state. Use when the user asks for a list of all areas, 'alle Bereiche', or 'alle Areas' in the launcher.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
            handler: Box::new(|sender, _params| {
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::ListAllAreas {
                            response: oneshot::channel().0,
                        },
                    )
                    .await
                })
            }),
        },
        ToolDefinition {
            name: "get_area_config".to_string(),
            description: "Returns the full configuration of a Smearor area (Bereich) as JSON, including its buttons and their associated actions. Use this after listing areas to inspect the devices or controls available inside a specific area.".to_string(),
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
        ToolDefinition {
            name: "launcher_load_instance".to_string(),
            description: "Dynamically loads a new launcher instance from a TOML config file path. The instance gets its own window, plugins, and areas. Use this to add a new launcher window at runtime.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {
                        "type": "string",
                        "description": "Unique identifier for the new instance (e.g. 'side3', 'macropad_5')"
                    },
                    "config_path": {
                        "type": "string",
                        "description": "File system path to the TOML config file (e.g. 'config-side3.toml')"
                    },
                    "instance_type": {
                        "type": "string",
                        "enum": ["gtk", "headless", "web"],
                        "default": "gtk",
                        "description": "Instance type: 'gtk' creates a visible window, 'headless' runs without a window (for hardware devices), 'web' serves the instance via HTTP"
                    },
                    "persist": {
                        "type": "boolean",
                        "default": false,
                        "description": "Whether to persist this instance to the state file so it survives restarts. Set to true for config-file instances, false for transient runtime instances."
                    }
                },
                "required": ["instance_id", "config_path"]
            }),
            handler: Box::new(|sender, params| {
                let Some(instance_id) = get_string_param(params, "instance_id") else {
                    return Box::pin(async move { Err("Missing instance_id".to_string()) }) as ToolFuture;
                };
                let Some(config_path) = get_string_param(params, "config_path") else {
                    return Box::pin(async move { Err("Missing config_path".to_string()) }) as ToolFuture;
                };
                let instance_type = get_string_param(params, "instance_type").unwrap_or_else(|| "gtk".to_string());
                let persist = params.and_then(|p| p.get("persist")).and_then(|v| v.as_bool()).unwrap_or(false);
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::LoadInstance {
                            instance_id,
                            config_path,
                            instance_type,
                            persist,
                            response: oneshot::channel().0,
                        },
                    )
                        .await
                })
            }),
        },
        ToolDefinition {
            name: "launcher_start_instance".to_string(),
            description: "Starts a loaded (Ready) launcher instance by its instance_id. Builds the window or headless areas and transitions the instance to Running state. If the instance is already running, this is a no-op.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {
                        "type": "string",
                        "description": "Unique identifier of the instance to start"
                    }
                },
                "required": ["instance_id"]
            }),
            handler: Box::new(|sender, params| {
                let Some(instance_id) = get_string_param(params, "instance_id") else {
                    return Box::pin(async move { Err("Missing instance_id".to_string()) }) as ToolFuture;
                };
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::StartInstance {
                            instance_id,
                            response: oneshot::channel().0,
                        },
                    )
                        .await
                })
            }),
        },
        ToolDefinition {
            name: "launcher_stop_instance".to_string(),
            description: "Stops a running launcher instance by its instance_id. Closes the window and transitions the instance to Ready state. The instance remains loaded and can be started again. Other instances are not affected.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {
                        "type": "string",
                        "description": "Unique identifier of the instance to stop"
                    }
                },
                "required": ["instance_id"]
            }),
            handler: Box::new(|sender, params| {
                let Some(instance_id) = get_string_param(params, "instance_id") else {
                    return Box::pin(async move { Err("Missing instance_id".to_string()) }) as ToolFuture;
                };
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::StopInstance {
                            instance_id,
                            response: oneshot::channel().0,
                        },
                    )
                        .await
                })
            }),
        },
        ToolDefinition {
            name: "launcher_unload_instance".to_string(),
            description: "Unloads a launcher instance entirely by its instance_id. If the instance is running, it is stopped first. Then removes plugins, watchers, persistence, and frees the instance ID.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {
                        "type": "string",
                        "description": "Unique identifier of the instance to unload"
                    }
                },
                "required": ["instance_id"]
            }),
            handler: Box::new(|sender, params| {
                let Some(instance_id) = get_string_param(params, "instance_id") else {
                    return Box::pin(async move { Err("Missing instance_id".to_string()) }) as ToolFuture;
                };
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::UnloadInstance {
                            instance_id,
                            response: oneshot::channel().0,
                        },
                    )
                        .await
                })
            }),
        },
        ToolDefinition {
            name: "launcher_reload_instance".to_string(),
            description: "Hot-reloads a launcher instance by its instance_id. Stops the instance if running, unloads it, re-loads from its config file, and restores the previous lifecycle state (Running or Ready).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {
                        "type": "string",
                        "description": "Unique identifier of the instance to reload"
                    },
                    "config_path": {
                        "type": "string",
                        "description": "Optional path to a new config file. If omitted, the original config path is reused."
                    }
                },
                "required": ["instance_id"]
            }),
            handler: Box::new(|sender, params| {
                let Some(instance_id) = get_string_param(params, "instance_id") else {
                    return Box::pin(async move { Err("Missing instance_id".to_string()) }) as ToolFuture;
                };
                let config_path = get_string_param(params, "config_path").unwrap_or_default();
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::ReloadInstance {
                            instance_id,
                            config_path,
                            response: oneshot::channel().0,
                        },
                    )
                        .await
                })
            }),
        },
        ToolDefinition {
            name: "launcher_list_instances".to_string(),
            description: "Lists all currently running launcher instances with their IDs, types, and window states.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
            handler: Box::new(|sender, _params| {
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::ListInstances {
                            response: oneshot::channel().0,
                        },
                    )
                        .await
                })
            }),
        },
        ToolDefinition {
            name: "web_server_status".to_string(),
            description: "Returns the status of the embedded web server, including port, enabled state, and list of active web instances.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
            handler: Box::new(|sender, _params| {
                Box::pin(async move {
                    send_command_and_wait(
                        sender,
                        McpCommand::WebServerStatus {
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
        McpCommand::ListAllAreas { response: _ } => McpCommand::ListAllAreas { response: response_tx },
        McpCommand::OpenTransientArea {
            area_id,
            source_area_id,
            response: _,
        } => McpCommand::OpenTransientArea {
            area_id,
            source_area_id,
            response: response_tx,
        },
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
        McpCommand::SendMultipleMessages { messages, response: _ } => McpCommand::SendMultipleMessages {
            messages,
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
        McpCommand::InvokePluginTool { .. } | McpCommand::InvokePluginResource { .. } | McpCommand::InvokePluginPrompt { .. } => {
            return Err("Plugin invocation commands are handled by the message handler".to_string());
        }
        McpCommand::LoadInstance {
            instance_id,
            config_path,
            instance_type,
            persist,
            response: _,
        } => McpCommand::LoadInstance {
            instance_id,
            config_path,
            instance_type,
            persist,
            response: response_tx,
        },
        McpCommand::StartInstance { instance_id, response: _ } => McpCommand::StartInstance {
            instance_id,
            response: response_tx,
        },
        McpCommand::StopInstance { instance_id, response: _ } => McpCommand::StopInstance {
            instance_id,
            response: response_tx,
        },
        McpCommand::UnloadInstance { instance_id, response: _ } => McpCommand::UnloadInstance {
            instance_id,
            response: response_tx,
        },
        McpCommand::ReloadInstance {
            instance_id,
            config_path,
            response: _,
        } => McpCommand::ReloadInstance {
            instance_id,
            config_path,
            response: response_tx,
        },
        McpCommand::ListInstances { response: _ } => McpCommand::ListInstances { response: response_tx },
        McpCommand::WebServerStatus { response: _ } => McpCommand::WebServerStatus { response: response_tx },
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
