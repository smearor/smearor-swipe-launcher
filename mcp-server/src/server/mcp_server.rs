use async_channel::Sender;
use rust_mcp_axum::AxumServerOptions;
use rust_mcp_axum::create_axum_server;
use rust_mcp_sdk::ToMcpServerHandler;
use rust_mcp_sdk::schema::Implementation;
use rust_mcp_sdk::schema::InitializeResult;
use rust_mcp_sdk::schema::ServerCapabilities;
use rust_mcp_sdk::schema::ServerCapabilitiesResources;
use rust_mcp_sdk::schema::ServerCapabilitiesTools;
use smearor_model_mcp::McpRegistry;
use std::sync::Arc;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::LogBuffer;
use crate::McpCommand;
use crate::server::McpServerConfig;
use crate::server::McpServerState;
use crate::server::SwipeLauncherHandler;

/// Builder for the MCP server.
pub struct McpServer {
    /// Server configuration: bind address, port, auth token.
    config: McpServerConfig,
    /// Channel for sending commands to the launcher core.
    command_sender: Sender<McpCommand>,
    /// Dynamic plugin registry shared with the server state.
    plugin_registry: McpRegistry,
    /// Shared log buffer for tracing log capture, or `None` when disabled.
    log_buffer: Option<Arc<LogBuffer>>,
    /// Handle to the spawned tokio task running the HTTP server.
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl McpServer {
    /// Create a new MCP server using an externally created command sender.
    pub fn new(config: McpServerConfig, plugin_registry: McpRegistry, command_sender: Sender<McpCommand>, log_buffer: Option<Arc<LogBuffer>>) -> Self {
        Self {
            config,
            command_sender,
            plugin_registry,
            log_buffer,
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

        let state = Arc::new(
            McpServerState::builder()
                .command_sender(self.command_sender.clone())
                .plugin_registry(self.plugin_registry.clone())
                .log_buffer(self.log_buffer.clone())
                .build(),
        );

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
