//! MCP server for the Smearor Swipe Launcher.
//!
//! Exposes launcher control and state through the Model Context Protocol using
//! the `rust-mcp-sdk` and `rust-mcp-axum` crates for robust protocol handling
//! with Streamable HTTP and SSE transport support.

use async_channel::Sender;
use async_trait::async_trait;
use rust_mcp_axum::AxumServerOptions;
use rust_mcp_axum::create_axum_server;
use rust_mcp_sdk::ToMcpServerHandler;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::CallToolRequestParams;
use rust_mcp_sdk::schema::CallToolResult;
use rust_mcp_sdk::schema::GetPromptRequestParams;
use rust_mcp_sdk::schema::GetPromptResult;
use rust_mcp_sdk::schema::Implementation;
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
use rust_mcp_sdk::schema::ServerCapabilities;
use rust_mcp_sdk::schema::ServerCapabilitiesResources;
use rust_mcp_sdk::schema::ServerCapabilitiesTools;
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::TextResourceContents;
use rust_mcp_sdk::schema::Tool;
use rust_mcp_sdk::schema::ToolInputSchema;
use rust_mcp_sdk::schema::schema_utils::CallToolError;
use smearor_model_mcp::McpRegistry;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use tokio::sync::oneshot;
use tracing::error;
use tracing::info;
use tracing::warn;

pub mod command;
mod jsonrpc;
pub mod prompts;
pub mod resources;
pub mod tools;

pub use command::CloseAreaParams;
pub use command::CommandResponseWrapper;
pub use command::FocusAreaParams;
pub use command::GetAreaConfigParams;
pub use command::InstanceTypeParam;
pub use command::InvokePluginPromptParams;
pub use command::InvokePluginResourceParams;
pub use command::InvokePluginToolParams;
pub use command::ListAllAreasParams;
pub use command::ListAreasParams;
pub use command::ListInstancesParams;
pub use command::LoadInstanceParams;
pub use command::McpCommand;
pub use command::McpCommandVariant;
pub use command::OpenAreaParams;
pub use command::OpenTransientAreaParams;
pub use command::ReadResourceParams;
pub use command::ReloadInstanceParams;
pub use command::SendMessageParams;
pub use command::SendMultipleMessagesParams;
pub use command::StartInstanceParams;
pub use command::StopInstanceParams;
pub use command::ToggleAreaParams;
pub use command::UnloadInstanceParams;
pub use command::WebServerStatusParams;

/// Configuration for the MCP server.
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    /// Address to bind the HTTP server to.
    pub bind_address: String,
    /// TCP port to listen on.
    pub port: u16,
    /// Optional bearer token required for all HTTP requests.
    pub auth_token: Option<String>,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 8765,
            auth_token: None,
        }
    }
}

/// Shared state used by the ServerHandler to process MCP requests.
/// This bridges the SDK's handler trait with the existing McpCommand channel
/// system for communication with the launcher core.
pub struct McpServerState {
    /// Channel used by tool/resource handlers to request actions from the
    /// launcher core.
    pub command_sender: Sender<McpCommand>,
    /// Registered core tools.
    pub tools: Vec<tools::ToolDefinition>,
    /// Registered core resources.
    pub resources: Vec<resources::ResourceDefinition>,
    /// Registered core prompts.
    pub prompts: Vec<prompts::PromptDefinition>,
    /// Dynamic registry populated by plugins.
    pub plugin_registry: McpRegistry,
    /// Monotonic counter for MCP invocation correlation IDs.
    pub correlation_counter: AtomicU64,
}

/// Builder for the MCP server.
pub struct McpServer {
    config: McpServerConfig,
    command_sender: Sender<McpCommand>,
    plugin_registry: McpRegistry,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl McpServer {
    /// Create a new MCP server using an externally created command sender.
    pub fn new(config: McpServerConfig, plugin_registry: McpRegistry, command_sender: async_channel::Sender<McpCommand>) -> Self {
        Self {
            config,
            command_sender,
            plugin_registry,
            task_handle: None,
        }
    }

    /// Start the MCP server using rust-mcp-axum's AxumServer in a spawned
    /// tokio task. The server supports both Streamable HTTP and SSE transports.
    pub fn start(&mut self) {
        if self.task_handle.is_some() {
            warn!("MCP server already running");
            return;
        }

        let state = Arc::new(McpServerState {
            command_sender: self.command_sender.clone(),
            tools: tools::core_tools(),
            resources: resources::core_resources(),
            prompts: prompts::core_prompts(),
            plugin_registry: self.plugin_registry.clone(),
            correlation_counter: AtomicU64::new(1),
        });

        let handler = SwipeLauncherHandler {
            state,
            server_details: Self::initialize_result(),
        };

        let server_options = AxumServerOptions {
            host: self.config.bind_address.clone(),
            port: self.config.port,
            sse_support: true,
            enable_json_response: Some(true),
            ..Default::default()
        };

        let handler_arc = handler.to_mcp_server_handler();
        let server = create_axum_server(Self::initialize_result(), handler_arc, server_options);

        info!("MCP server starting on {}:{}", self.config.bind_address, self.config.port);
        let handle = tokio::spawn(async move {
            if let Err(e) = server.start().await {
                error!("MCP server error: {:?}", e);
            }
        });
        self.task_handle = Some(handle);
    }

    /// Stop the running MCP server.
    pub fn stop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        info!("MCP server stopped");
    }

    /// Build the InitializeResult that advertises server capabilities.
    fn initialize_result() -> InitializeResult {
        InitializeResult {
            protocol_version: "2025-11-25".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ServerCapabilitiesTools { list_changed: Some(true) }),
                resources: Some(ServerCapabilitiesResources {
                    list_changed: Some(true),
                    subscribe: Some(true),
                }),
                prompts: Some(rust_mcp_sdk::schema::ServerCapabilitiesPrompts { list_changed: Some(true) }),
                ..Default::default()
            },
            instructions: None,
            meta: None,
            server_info: Implementation {
                name: "smearor-mcp-server".to_string(),
                version: "0.1.0".to_string(),
                title: None,
                description: None,
                icons: vec![],
                website_url: None,
            },
        }
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// ServerHandler implementation that bridges rust-mcp-sdk with the existing
/// McpCommand channel system for launcher core and plugin communication.
pub struct SwipeLauncherHandler {
    state: Arc<McpServerState>,
    server_details: InitializeResult,
}

#[async_trait]
impl ServerHandler for SwipeLauncherHandler {
    async fn handle_initialize_request(
        &self,
        _params: InitializeRequestParams,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<InitializeResult, RpcError> {
        Ok(self.server_details.clone())
    }

    async fn handle_ping_request(
        &self,
        _params: Option<rust_mcp_sdk::schema::RequestParams>,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<McpResult, RpcError> {
        Ok(McpResult::default())
    }

    async fn handle_list_tools_request(
        &self,
        _params: Option<rust_mcp_sdk::schema::PaginatedRequestParams>,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        let state = self.state.clone();
        let mut sdk_tools: Vec<Tool> = state
            .tools
            .iter()
            .map(|t| Tool {
                name: t.name.clone(),
                description: Some(t.description.clone()),
                input_schema: json_schema_to_tool_input_schema(&t.input_schema),
                annotations: None,
                execution: None,
                icons: vec![],
                meta: None,
                output_schema: None,
                title: None,
            })
            .collect();
        for plugin_tool in state.plugin_registry.list_tools() {
            sdk_tools.push(Tool {
                name: plugin_tool.name.clone(),
                description: Some(plugin_tool.description.clone()),
                input_schema: json_schema_to_tool_input_schema(&plugin_tool.input_schema),
                annotations: None,
                execution: None,
                icons: vec![],
                meta: None,
                output_schema: None,
                title: None,
            });
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
    ) -> std::result::Result<CallToolResult, CallToolError> {
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
        let result = tools::invoke_tool_sdk(&state.tools, state.command_sender.clone(), &name, Some(&arguments_value)).await;
        match result {
            Ok(text) => Ok(CallToolResult::text_content(vec![TextContent::new(text, None, None)])),
            Err(message) => Ok(CallToolResult::with_error(CallToolError::from_message(message))),
        }
    }

    async fn handle_list_resources_request(
        &self,
        _params: Option<rust_mcp_sdk::schema::PaginatedRequestParams>,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<ListResourcesResult, RpcError> {
        let state = self.state.clone();
        let mut sdk_resources: Vec<Resource> = state
            .resources
            .iter()
            .map(|r| Resource {
                uri: r.uri.clone(),
                name: r.name.clone(),
                description: Some(r.description.clone()),
                mime_type: Some(r.mime_type.clone()),
                annotations: None,
                icons: vec![],
                meta: None,
                size: None,
                title: None,
            })
            .collect();
        for plugin_resource in state.plugin_registry.list_resources() {
            sdk_resources.push(Resource {
                uri: plugin_resource.uri.clone(),
                name: plugin_resource.name.clone(),
                description: Some(plugin_resource.description.clone()),
                mime_type: Some(plugin_resource.mime_type.clone()),
                annotations: None,
                icons: vec![],
                meta: None,
                size: None,
                title: None,
            });
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
    ) -> std::result::Result<ListResourceTemplatesResult, RpcError> {
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
    ) -> std::result::Result<ReadResourceResult, RpcError> {
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
        match resources::read_resource_sdk(&state.resources, state.command_sender.clone(), &uri).await {
            Ok((contents, mime_type)) => Ok(ReadResourceResult {
                contents: vec![ReadResourceContent::TextResourceContents(TextResourceContents {
                    meta: None,
                    mime_type: Some(mime_type),
                    text: contents,
                    uri,
                })],
                meta: None,
            }),
            Err(message) => Err(RpcError::internal_error().with_message(message)),
        }
    }

    async fn handle_list_prompts_request(
        &self,
        _params: Option<rust_mcp_sdk::schema::PaginatedRequestParams>,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<ListPromptsResult, RpcError> {
        let state = self.state.clone();
        let mut sdk_prompts: Vec<rust_mcp_sdk::schema::Prompt> = state.prompts.iter().map(prompts::prompt_to_sdk).collect();
        for plugin_prompt in state.plugin_registry.list_prompts() {
            sdk_prompts.push(rust_mcp_sdk::schema::Prompt {
                name: plugin_prompt.name.clone(),
                description: Some(plugin_prompt.description.clone()),
                arguments: prompts::schema_to_prompt_arguments(&plugin_prompt.arguments_schema),
                icons: vec![],
                meta: None,
                title: None,
            });
        }
        Ok(ListPromptsResult {
            prompts: sdk_prompts,
            next_cursor: None,
            meta: None,
        })
    }

    async fn handle_get_prompt_request(
        &self,
        params: GetPromptRequestParams,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<GetPromptResult, RpcError> {
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

        match prompts::get_prompt_sdk(&state.prompts, &name, &arguments) {
            Ok(result) => Ok(result),
            Err(message) => Err(RpcError::internal_error().with_message(message)),
        }
    }
}

/// Convert a serde_json::Value JSON schema to the SDK's ToolInputSchema.
/// The input schema must be a JSON object with "properties" and "required" fields.
/// Properties are converted from serde_json::Map to BTreeMap<String, serde_json::Map>.
fn json_schema_to_tool_input_schema(schema: &serde_json::Value) -> ToolInputSchema {
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
