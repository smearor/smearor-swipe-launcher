use crate::instance::LauncherInstance;
use crate::web::template::TemplateEngine;
use crate::web::ws_manager::WebSocketManager;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;

/// Shared application state for the web server.
///
/// # Safety
///
/// `LauncherInstance` contains raw pointers (`LoadedPlugin` has `*mut c_void`
/// and `*const WidgetPluginVTable`). Access to `instances` is protected by a `Mutex`,
/// and raw pointers are only dereferenced inside `unsafe` blocks with proper
/// lifetime guarantees. The `broker_sender` is `Send + Sync`.
pub struct WebAppState {
    /// All loaded launcher instances, keyed by instance ID.
    pub instances: Arc<Mutex<HashMap<String, LauncherInstance>>>,
    /// Sender for forwarding broker messages to the message routing pipeline.
    #[allow(dead_code)]
    pub broker_sender: UnboundedSender<FfiEnvelope>,
    /// Template engine for rendering web instance HTML pages.
    pub template_engine: TemplateEngine,
    /// Manager for per-instance WebSocket broadcast channels.
    pub ws_manager: Arc<WebSocketManager>,
    /// Sender for MCP commands (instance lifecycle control via REST API).
    pub mcp_command_sender: async_channel::Sender<smearor_mcp_server::McpCommand>,
}

unsafe impl Send for WebAppState {}
unsafe impl Sync for WebAppState {}
