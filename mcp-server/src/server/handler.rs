use async_trait::async_trait;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::CallToolRequestParams;
use rust_mcp_sdk::schema::CallToolResult;
use rust_mcp_sdk::schema::GetPromptRequestParams;
use rust_mcp_sdk::schema::GetPromptResult;
use rust_mcp_sdk::schema::InitializeRequestParams;
use rust_mcp_sdk::schema::InitializeResult;
use rust_mcp_sdk::schema::ListPromptsResult;
use rust_mcp_sdk::schema::ListResourceTemplatesResult;
use rust_mcp_sdk::schema::ListResourcesResult;
use rust_mcp_sdk::schema::ListToolsResult;
use rust_mcp_sdk::schema::ReadResourceContent;
use rust_mcp_sdk::schema::ReadResourceRequestParams;
use rust_mcp_sdk::schema::ReadResourceResult;
use rust_mcp_sdk::schema::Resource;
use rust_mcp_sdk::schema::Result as McpResult;
use rust_mcp_sdk::schema::RpcError;
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::TextResourceContents;
use rust_mcp_sdk::schema::Tool;
use rust_mcp_sdk::schema::schema_utils::CallToolError;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::oneshot;

use crate::CommandResponseWrapper;
use crate::InvokePluginPromptParams;
use crate::InvokePluginResourceParams;
use crate::InvokePluginToolParams;
use crate::prompts::IntoSdkPrompt;
use crate::prompts::get_prompt_sdk;
use crate::resources::IntoSdkResource;
use crate::resources::read_resource_sdk;
use crate::server::McpServerState;
use crate::tools::IntoSdkTool;
use crate::tools::ToolInvocation;
use crate::tools::invoke_tool_sdk;

/// ServerHandler implementation that bridges rust-mcp-sdk with the existing
/// McpCommand channel system for launcher core and plugin communication.
pub struct SwipeLauncherHandler {
    /// Shared server state containing tools, resources, prompts and command channel.
    pub state: Arc<McpServerState>,
    /// Cached InitializeResult advertised to clients on handshake.
    pub server_details: InitializeResult,
}

#[async_trait]
impl ServerHandler for SwipeLauncherHandler {
    async fn handle_initialize_request(
        &self,
        _params: InitializeRequestParams,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> Result<InitializeResult, RpcError> {
        Ok(self.server_details.clone())
    }

    async fn handle_ping_request(
        &self,
        _params: Option<rust_mcp_sdk::schema::RequestParams>,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> Result<McpResult, RpcError> {
        Ok(McpResult::default())
    }

    async fn handle_list_resources_request(
        &self,
        _params: Option<rust_mcp_sdk::schema::PaginatedRequestParams>,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> Result<ListResourcesResult, RpcError> {
        let state = self.state.clone();
        let mut sdk_resources: Vec<Resource> = state.resources.iter().map(|r| r.into_sdk_resource()).collect();
        for plugin_resource in state.plugin_registry.list_resources() {
            sdk_resources.push(plugin_resource.into_sdk_resource());
        }
        Ok(ListResourcesResult {
            resources: sdk_resources,
            next_cursor: None,
            meta: None,
        })
    }

    async fn handle_list_resource_templates_request(
        &self,
        _params: Option<rust_mcp_sdk::schema::PaginatedRequestParams>,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> Result<ListResourceTemplatesResult, RpcError> {
        Ok(ListResourceTemplatesResult {
            resource_templates: vec![],
            next_cursor: None,
            meta: None,
        })
    }

    async fn handle_read_resource_request(
        &self,
        params: ReadResourceRequestParams,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> Result<ReadResourceResult, RpcError> {
        let state = self.state.clone();
        let uri = params.uri.clone();

        // Check if it's a plugin resource
        if let Some(plugin_resource) = state.plugin_registry.list_resources().iter().find(|r| r.uri == uri).cloned() {
            let correlation_id = state.correlation_counter.fetch_add(1, Ordering::Relaxed).to_string();
            let (response_tx, response_rx) = oneshot::channel::<Result<String, String>>();
            let _ = state.command_sender.try_send(
                CommandResponseWrapper::builder()
                    .params(
                        InvokePluginResourceParams::builder()
                            .uri(plugin_resource.uri.clone())
                            .plugin_id(plugin_resource.plugin_id.clone())
                            .correlation_id(correlation_id)
                            .build(),
                    )
                    .response(response_tx)
                    .build()
                    .into(),
            );
            match tokio::time::timeout(tokio::time::Duration::from_secs(10), response_rx).await {
                Ok(Ok(Ok(contents))) => {
                    return Ok(ReadResourceResult {
                        contents: vec![ReadResourceContent::TextResourceContents(TextResourceContents {
                            meta: None,
                            mime_type: Some(plugin_resource.mime_type.clone()),
                            text: contents,
                            uri: plugin_resource.uri.clone(),
                        })],
                        meta: None,
                    });
                }
                Ok(Ok(Err(message))) => {
                    return Err(RpcError::internal_error().with_message(message));
                }
                Ok(Err(_)) => {
                    return Err(RpcError::internal_error().with_message("Plugin resource read dropped".to_string()));
                }
                Err(_) => {
                    return Err(RpcError::internal_error().with_message("Plugin resource read timed out".to_string()));
                }
            }
        }

        // Core resource
        match read_resource_sdk(&state.resources, state.command_sender.clone(), &uri).await {
            Ok(output) => Ok(ReadResourceResult {
                contents: vec![ReadResourceContent::TextResourceContents(TextResourceContents {
                    meta: None,
                    mime_type: Some(output.mime_type),
                    text: output.contents,
                    uri,
                })],
                meta: None,
            }),
            Err(e) => Err(RpcError::internal_error().with_message(e.to_string())),
        }
    }

    async fn handle_list_prompts_request(
        &self,
        _params: Option<rust_mcp_sdk::schema::PaginatedRequestParams>,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> Result<ListPromptsResult, RpcError> {
        let state = self.state.clone();
        let mut sdk_prompts: Vec<rust_mcp_sdk::schema::Prompt> = state.prompts.iter().map(|p| p.into_sdk_prompt()).collect();
        for plugin_prompt in state.plugin_registry.list_prompts() {
            sdk_prompts.push(plugin_prompt.into_sdk_prompt());
        }
        Ok(ListPromptsResult {
            prompts: sdk_prompts,
            next_cursor: None,
            meta: None,
        })
    }

    async fn handle_get_prompt_request(&self, params: GetPromptRequestParams, _runtime: Arc<dyn rust_mcp_sdk::McpServer>) -> Result<GetPromptResult, RpcError> {
        let state = self.state.clone();
        let name = params.name.clone();
        let arguments = params.arguments.clone();

        if let Some(plugin_prompt) = state.plugin_registry.list_prompts().into_iter().find(|p| p.name == name) {
            let correlation_id = state.correlation_counter.fetch_add(1, Ordering::Relaxed).to_string();
            let (response_tx, response_rx) = oneshot::channel::<Result<String, String>>();
            let arguments_value = arguments
                .map(|m| {
                    let map: serde_json::Map<String, serde_json::Value> = m.into_iter().map(|(k, v)| (k, serde_json::Value::String(v))).collect();
                    serde_json::Value::Object(map)
                })
                .unwrap_or(serde_json::Value::Null);
            let _ = state.command_sender.try_send(
                CommandResponseWrapper::builder()
                    .params(
                        InvokePluginPromptParams::builder()
                            .name(plugin_prompt.name.clone())
                            .plugin_id(plugin_prompt.plugin_id.clone())
                            .correlation_id(correlation_id)
                            .arguments(arguments_value)
                            .build(),
                    )
                    .response(response_tx)
                    .build()
                    .into(),
            );
            match tokio::time::timeout(tokio::time::Duration::from_secs(10), response_rx).await {
                Ok(Ok(Ok(json))) => {
                    let result: GetPromptResult = serde_json::from_str(&json).unwrap_or(GetPromptResult {
                        description: None,
                        messages: vec![],
                        meta: None,
                    });
                    return Ok(result);
                }
                Ok(Ok(Err(message))) => {
                    return Err(RpcError::internal_error().with_message(message));
                }
                Ok(Err(_)) => {
                    return Err(RpcError::internal_error().with_message("Plugin prompt invocation dropped"));
                }
                Err(_) => {
                    return Err(RpcError::internal_error().with_message("Plugin prompt invocation timed out"));
                }
            }
        }

        match get_prompt_sdk(&state.prompts, &name, &arguments) {
            Ok(result) => Ok(result),
            Err(e) => Err(RpcError::internal_error().with_message(e.to_string())),
        }
    }

    async fn handle_list_tools_request(
        &self,
        _params: Option<rust_mcp_sdk::schema::PaginatedRequestParams>,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> Result<ListToolsResult, RpcError> {
        let state = self.state.clone();
        let mut sdk_tools: Vec<Tool> = state.tools.iter().map(|t| t.into_sdk_tool()).collect();
        for plugin_tool in state.plugin_registry.list_tools() {
            sdk_tools.push(plugin_tool.into_sdk_tool());
        }
        Ok(ListToolsResult {
            tools: sdk_tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> Result<CallToolResult, CallToolError> {
        let state = self.state.clone();
        let name = params.name.clone();
        let arguments = params.arguments.clone();

        // Check if it's a plugin tool
        if let Some(plugin_tool) = state.plugin_registry.list_tools().iter().find(|t| t.name == name).cloned() {
            let correlation_id = state.correlation_counter.fetch_add(1, Ordering::Relaxed).to_string();
            let (response_tx, response_rx) = oneshot::channel::<Result<String, String>>();
            let arguments_value = arguments.map(|m| serde_json::Value::Object(m)).unwrap_or(serde_json::Value::Null);
            let _ = state.command_sender.try_send(
                CommandResponseWrapper::builder()
                    .params(
                        InvokePluginToolParams::builder()
                            .name(plugin_tool.name.clone())
                            .plugin_id(plugin_tool.plugin_id.clone())
                            .correlation_id(correlation_id)
                            .arguments(arguments_value)
                            .build(),
                    )
                    .response(response_tx)
                    .build()
                    .into(),
            );
            match tokio::time::timeout(tokio::time::Duration::from_secs(10), response_rx).await {
                Ok(Ok(Ok(result))) => {
                    return Ok(CallToolResult::text_content(vec![TextContent::new(result, None, None)]));
                }
                Ok(Ok(Err(message))) => {
                    return Ok(CallToolResult::with_error(CallToolError::from_message(message)));
                }
                Ok(Err(_)) => {
                    return Ok(CallToolResult::with_error(CallToolError::from_message("Plugin tool invocation dropped")));
                }
                Err(_) => {
                    return Ok(CallToolResult::with_error(CallToolError::from_message("Plugin tool invocation timed out")));
                }
            }
        }

        // Core tool
        let arguments_value = arguments.map(serde_json::Value::Object).unwrap_or(serde_json::Value::Null);
        let invocation = ToolInvocation::new(state.command_sender.clone(), Some(&arguments_value));
        let result = invoke_tool_sdk(&state.tools, invocation, &name).await;
        match result {
            Ok(text) => Ok(CallToolResult::text_content(vec![TextContent::new(text, None, None)])),
            Err(e) => Ok(CallToolResult::with_error(CallToolError::from_message(e.to_string()))),
        }
    }
}
