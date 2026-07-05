//! MCP server for the Smearor Swipe Launcher.
//!
//! Exposes launcher control and state through the Model Context Protocol over
//! an axum-based SSE transport. The server is designed to run as a dedicated
//! task inside the launcher process and communicates with the core through a
//! command channel.

use async_channel::Sender;
use axum::Router;
use axum::extract::OriginalUri;
use axum::extract::Request;
use axum::extract::State;
use axum::middleware;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::response::sse::Event;
use axum::response::sse::Sse;
use axum::routing::get;
use axum::routing::post;
use futures_util::Stream;
use smearor_model_mcp::McpRegistry;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use tower_http::cors::CorsLayer;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

mod jsonrpc;
mod resources;
mod tools;

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

/// Shared state of the running MCP server.
#[derive(Clone)]
pub struct McpServerState {
    /// Channel used by tool/resource handlers to request actions from the
    /// launcher core.
    command_sender: Sender<McpCommand>,
    /// Registered core tools.
    tools: Arc<Vec<tools::ToolDefinition>>,
    /// Registered core resources.
    resources: Arc<Vec<resources::ResourceDefinition>>,
    /// Dynamic registry populated by plugins.
    plugin_registry: McpRegistry,
    /// Monotonic counter for MCP invocation correlation IDs.
    correlation_counter: Arc<AtomicU64>,
    /// Optional bearer token required for all HTTP requests.
    auth_token: Option<String>,
    /// Sender used by the launcher core to push JSON-RPC notifications to
    /// all connected SSE clients.
    notification_sender: broadcast::Sender<String>,
    /// Manages per-session channels for routing JSON-RPC responses to the
    /// SSE client that issued the request.
    session_manager: SessionManager,
}

/// Routes JSON-RPC responses to a specific SSE session.
#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
}

impl SessionManager {
    /// Create a new empty session manager.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new session, returning its ID and the receiver for that session.
    pub fn create_session(&self) -> (String, broadcast::Receiver<String>) {
        let session_id = uuid::Uuid::new_v4().to_string();
        let (sender, receiver) = broadcast::channel::<String>(16);
        self.sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(session_id.clone(), sender);
        (session_id, receiver)
    }

    /// Send a message to a specific session. If the session does not exist,
    /// the message is silently dropped.
    pub fn send_to_session(&self, session_id: &str, message: String) {
        if let Some(sender) = self.sessions.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).get(session_id) {
            let _ = sender.send(message);
        }
    }

    /// Remove a session from the manager.
    pub fn remove_session(&self, session_id: &str) {
        self.sessions.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(session_id);
    }

    /// Check whether a session is currently registered.
    pub fn has_session(&self, session_id: &str) -> bool {
        self.sessions.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).contains_key(session_id)
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Commands sent from the MCP server to the launcher core.
pub enum McpCommand {
    /// Open an area by ID.
    OpenArea {
        area_id: String,
        response: oneshot::Sender<Result<String, String>>,
    },
    /// Close an area by ID.
    CloseArea {
        area_id: String,
        response: oneshot::Sender<Result<String, String>>,
    },
    /// List all configured areas.
    ListAreas { response: oneshot::Sender<Result<String, String>> },
    /// Focus an area by ID.
    FocusArea {
        area_id: String,
        response: oneshot::Sender<Result<String, String>>,
    },
    /// Send a message to a broker topic.
    SendMessage {
        topic: String,
        payload: serde_json::Value,
        target_instance_id: Option<String>,
        response: oneshot::Sender<Result<String, String>>,
    },
    /// Read a resource by URI.
    ReadResource {
        uri: String,
        response: oneshot::Sender<Result<String, String>>,
    },
    /// Toggle the visibility of an area.
    ToggleArea {
        area_id: String,
        response: oneshot::Sender<Result<String, String>>,
    },
    /// Get the configuration of an area as JSON.
    GetAreaConfig {
        area_id: String,
        response: oneshot::Sender<Result<String, String>>,
    },
    /// Invoke a tool registered by a plugin.
    InvokePluginTool {
        name: String,
        plugin_id: String,
        correlation_id: String,
        arguments: serde_json::Value,
        response: oneshot::Sender<Result<String, String>>,
    },
    /// Read a resource registered by a plugin.
    InvokePluginResource {
        uri: String,
        plugin_id: String,
        correlation_id: String,
        response: oneshot::Sender<Result<String, String>>,
    },
}

/// Builder for the MCP server.
pub struct McpServer {
    config: McpServerConfig,
    command_sender: Sender<McpCommand>,
    plugin_registry: McpRegistry,
    auth_token: Option<String>,
    /// Channel used by the launcher core to push notifications to connected
    /// SSE clients.
    notification_sender: broadcast::Sender<String>,
    shutdown_sender: Option<oneshot::Sender<()>>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl McpServer {
    /// Create a new MCP server and the receiver that the launcher core will
    /// use to consume commands.
    pub fn new(config: McpServerConfig, plugin_registry: McpRegistry) -> (Self, async_channel::Receiver<McpCommand>, broadcast::Sender<String>) {
        let (command_sender, receiver) = async_channel::unbounded::<McpCommand>();
        let (notification_sender, _) = broadcast::channel::<String>(16);
        let auth_token = config.auth_token.clone();
        let server = Self {
            config,
            command_sender,
            plugin_registry,
            auth_token,
            notification_sender: notification_sender.clone(),
            shutdown_sender: None,
            task_handle: None,
        };
        (server, receiver, notification_sender)
    }

    /// Start the axum HTTP server in a spawned tokio task.
    ///
    /// The server keeps running until [`McpServer::stop`] is called.
    pub fn start(&mut self) {
        if self.task_handle.is_some() {
            warn!("MCP server already running");
            return;
        }

        let (shutdown_sender, shutdown_receiver) = oneshot::channel::<()>();
        self.shutdown_sender = Some(shutdown_sender);

        let state = McpServerState {
            command_sender: self.command_sender.clone(),
            tools: Arc::new(tools::core_tools()),
            resources: Arc::new(resources::core_resources()),
            plugin_registry: self.plugin_registry.clone(),
            correlation_counter: Arc::new(AtomicU64::new(1)),
            auth_token: self.auth_token.clone(),
            notification_sender: self.notification_sender.clone(),
            session_manager: SessionManager::new(),
        };

        let bind_addr = self.bind_addr();
        let handle = tokio::spawn(run_server(bind_addr, state, shutdown_receiver));
        self.task_handle = Some(handle);
        info!("MCP server started on {}", bind_addr);
    }

    /// Stop the running MCP server.
    pub fn stop(&mut self) {
        if let Some(sender) = self.shutdown_sender.take() {
            let _ = sender.send(());
        }
        if let Some(handle) = self.task_handle.take() {
            tokio::spawn(async move {
                if let Err(e) = handle.await {
                    error!("MCP server task failed: {}", e);
                }
            });
        }
        info!("MCP server stopped");
    }

    fn bind_addr(&self) -> SocketAddr {
        let addr = self
            .config
            .bind_address
            .parse()
            .unwrap_or_else(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
        SocketAddr::new(addr, self.config.port)
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        self.stop();
    }
}

async fn auth_middleware(State(state): State<Arc<McpServerState>>, request: Request, next: Next) -> Response {
    if let Some(expected) = &state.auth_token {
        let header = request.headers().get("authorization").and_then(|value| value.to_str().ok()).unwrap_or("");
        let provided = header.strip_prefix("Bearer ").unwrap_or(header);
        if provided != expected {
            return (axum::http::StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
        }
    }
    next.run(request).await
}

async fn run_server(bind_addr: SocketAddr, state: McpServerState, shutdown_receiver: oneshot::Receiver<()>) {
    let app = Router::new()
        .route("/sse", get(sse_handler).post(streamable_http_handler))
        .route("/message", post(message_handler))
        .route("/streamable-http", post(streamable_http_handler))
        .route("/health", get(health_handler))
        .route_layer(middleware::from_fn_with_state(Arc::new(state.clone()), auth_middleware))
        .layer(CorsLayer::permissive())
        .with_state(Arc::new(state));

    let listener = match tokio::net::TcpListener::bind(bind_addr).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Failed to bind MCP server to {}: {}", bind_addr, e);
            return;
        }
    };

    let serve = axum::serve(listener, app);
    let graceful = serve.with_graceful_shutdown(async move {
        let _ = shutdown_receiver.await;
        debug!("MCP server graceful shutdown triggered");
    });

    if let Err(e) = graceful.await {
        error!("MCP server exited with error: {}", e);
    }
}

async fn health_handler() -> &'static str {
    "ok"
}

async fn message_handler(
    State(state): State<Arc<McpServerState>>,
    axum::extract::Query(query): axum::extract::Query<HashMap<String, String>>,
    OriginalUri(uri): OriginalUri,
    body: String,
) -> impl IntoResponse {
    let session_id = query.get("sessionId").or_else(|| query.get("session_id")).cloned();
    eprintln!(
        "DEBUG message_handler: uri={} query_keys={:?} session_id={:?}",
        uri,
        query.keys().collect::<Vec<_>>(),
        session_id
    );

    // SSE transport requires a valid session to route the response back. If
    // the client provides no session ID or an unknown one, return an error
    // immediately so the client does not wait for a response that can never
    // arrive on the SSE stream.
    let Some(session_id) = session_id else {
        let response = jsonrpc::JsonRpcResponse::error(None, jsonrpc::JSONRPC_INVALID_REQUEST, "Missing sessionId query parameter".to_string(), None);
        return (axum::http::StatusCode::BAD_REQUEST, axum::response::Json(serde_json::to_value(response).unwrap_or_default())).into_response();
    };
    if !state.session_manager.has_session(&session_id) {
        let response = jsonrpc::JsonRpcResponse::error(None, jsonrpc::JSONRPC_INVALID_REQUEST, format!("Unknown session ID: {}", session_id), None);
        return (axum::http::StatusCode::BAD_REQUEST, axum::response::Json(serde_json::to_value(response).unwrap_or_default())).into_response();
    }

    let request = match serde_json::from_str::<jsonrpc::JsonRpcRequest>(&body) {
        Ok(request) => request,
        Err(e) => {
            let response = jsonrpc::JsonRpcResponse::error(None, jsonrpc::JSONRPC_PARSE_ERROR, format!("Parse error: {}", e), None);
            // Legacy SSE transport: push parse errors to the requesting session.
            state
                .session_manager
                .send_to_session(&session_id, serde_json::to_string(&response).unwrap_or_default());
            return axum::http::StatusCode::ACCEPTED.into_response();
        }
    };

    // JSON-RPC notifications carry no id. The MCP protocol uses the
    // "notifications/" prefix for lifecycle notifications such as
    // "notifications/initialized". Acknowledge them without a response body.
    if request.id.is_none() {
        return axum::http::StatusCode::ACCEPTED.into_response();
    }

    let response = process_jsonrpc_request(state.clone(), request).await;
    let response_json = serde_json::to_string(&response).unwrap_or_default();
    state.session_manager.send_to_session(&session_id, response_json);
    axum::http::StatusCode::ACCEPTED.into_response()
}

async fn streamable_http_handler(State(state): State<Arc<McpServerState>>, body: String) -> impl IntoResponse {
    let request = match serde_json::from_str::<jsonrpc::JsonRpcRequest>(&body) {
        Ok(request) => request,
        Err(e) => {
            let response = jsonrpc::JsonRpcResponse::error(None, jsonrpc::JSONRPC_PARSE_ERROR, format!("Parse error: {}", e), None);
            return (axum::http::StatusCode::OK, axum::response::Json(serde_json::to_value(response).unwrap_or_default())).into_response();
        }
    };

    // MCP streamable-http: notifications (no id) are acknowledged with 202 Accepted.
    if request.id.is_none() {
        return axum::http::StatusCode::ACCEPTED.into_response();
    }

    let response = process_jsonrpc_request(state, request).await;
    axum::response::Json(serde_json::to_value(response).unwrap_or_default()).into_response()
}

async fn process_jsonrpc_request(state: Arc<McpServerState>, request: jsonrpc::JsonRpcRequest) -> jsonrpc::JsonRpcResponse {
    let id = request.id.clone();

    match request.method.as_str() {
        "tools/list" => {
            let mut tools = state
                .tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "inputSchema": t.input_schema
                    })
                })
                .collect::<Vec<_>>();
            for plugin_tool in state.plugin_registry.list_tools() {
                tools.push(serde_json::json!({
                    "name": plugin_tool.name,
                    "description": plugin_tool.description,
                    "inputSchema": plugin_tool.input_schema
                }));
            }
            jsonrpc::JsonRpcResponse::success(id, serde_json::json!({ "tools": tools }))
        }
        "tools/call" => {
            let params = request.params.as_ref();
            let name = jsonrpc::get_string_param(params, "name").unwrap_or_default();
            let arguments = jsonrpc::get_object_param(params, "arguments");
            if let Some(plugin_tool) = state.plugin_registry.list_tools().iter().find(|t| t.name == name) {
                let correlation_id = state.correlation_counter.fetch_add(1, Ordering::Relaxed).to_string();
                let (response_tx, response_rx) = oneshot::channel::<Result<String, String>>();
                let _ = state.command_sender.try_send(McpCommand::InvokePluginTool {
                    name: plugin_tool.name.clone(),
                    plugin_id: plugin_tool.plugin_id.clone(),
                    correlation_id: correlation_id.clone(),
                    arguments: arguments.unwrap_or(serde_json::Value::Null),
                    response: response_tx,
                });
                match tokio::time::timeout(tokio::time::Duration::from_secs(5), response_rx).await {
                    Ok(Ok(Ok(result))) => jsonrpc::JsonRpcResponse::success(id, serde_json::json!({ "content": [{ "type": "text", "text": result }] })),
                    Ok(Ok(Err(message))) => jsonrpc::JsonRpcResponse::error(id, jsonrpc::JSONRPC_INTERNAL_ERROR, message, None),
                    Ok(Err(_)) => jsonrpc::JsonRpcResponse::error(id, jsonrpc::JSONRPC_INTERNAL_ERROR, "Plugin tool invocation dropped".to_string(), None),
                    Err(_) => jsonrpc::JsonRpcResponse::error(id, jsonrpc::JSONRPC_INTERNAL_ERROR, "Plugin tool invocation timed out".to_string(), None),
                }
            } else {
                tools::invoke_tool(&state.tools, state.command_sender.clone(), id, &name, arguments.as_ref()).await
            }
        }
        "resources/list" => {
            let mut resources = state
                .resources
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "uri": r.uri,
                        "name": r.name,
                        "description": r.description,
                        "mimeType": r.mime_type
                    })
                })
                .collect::<Vec<_>>();
            for plugin_resource in state.plugin_registry.list_resources() {
                resources.push(serde_json::json!({
                    "uri": plugin_resource.uri,
                    "name": plugin_resource.name,
                    "description": plugin_resource.description,
                    "mimeType": plugin_resource.mime_type
                }));
            }
            jsonrpc::JsonRpcResponse::success(id, serde_json::json!({ "resources": resources }))
        }
        "resources/read" => {
            let params = request.params.as_ref();
            let uri = jsonrpc::get_string_param(params, "uri").unwrap_or_default();
            if let Some(plugin_resource) = state.plugin_registry.list_resources().iter().find(|r| r.uri == uri) {
                let correlation_id = state.correlation_counter.fetch_add(1, Ordering::Relaxed).to_string();
                let (response_tx, response_rx) = oneshot::channel::<Result<String, String>>();
                let _ = state.command_sender.try_send(McpCommand::InvokePluginResource {
                    uri: plugin_resource.uri.clone(),
                    plugin_id: plugin_resource.plugin_id.clone(),
                    correlation_id: correlation_id.clone(),
                    response: response_tx,
                });
                match tokio::time::timeout(tokio::time::Duration::from_secs(5), response_rx).await {
                    Ok(Ok(Ok(contents))) => jsonrpc::JsonRpcResponse::success(
                        id,
                        serde_json::json!({
                            "contents": [{
                                "uri": plugin_resource.uri,
                                "mimeType": plugin_resource.mime_type,
                                "text": contents
                            }]
                        }),
                    ),
                    Ok(Ok(Err(message))) => jsonrpc::JsonRpcResponse::error(id, jsonrpc::JSONRPC_INTERNAL_ERROR, message, None),
                    Ok(Err(_)) => jsonrpc::JsonRpcResponse::error(id, jsonrpc::JSONRPC_INTERNAL_ERROR, "Plugin resource read dropped".to_string(), None),
                    Err(_) => jsonrpc::JsonRpcResponse::error(id, jsonrpc::JSONRPC_INTERNAL_ERROR, "Plugin resource read timed out".to_string(), None),
                }
            } else {
                resources::read_resource_response(&state.resources, state.command_sender.clone(), id, uri).await
            }
        }
        "initialize" => jsonrpc::JsonRpcResponse::success(
            id,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": true },
                    "resources": { "listChanged": true, "subscribe": true }
                },
                "serverInfo": { "name": "smearor-mcp-server", "version": "0.1.0" }
            }),
        ),
        "initialized" => jsonrpc::JsonRpcResponse::success(id, serde_json::Value::Null),
        "ping" => jsonrpc::JsonRpcResponse::success(id, serde_json::Value::Null),
        _ => jsonrpc::JsonRpcResponse::error(id, jsonrpc::JSONRPC_METHOD_NOT_FOUND, format!("Method {} not found", request.method), None),
    }
}

async fn sse_handler(State(state): State<Arc<McpServerState>>, request: Request) -> Response {
    eprintln!("DEBUG sse_handler: uri={} headers={:?}", request.uri(), request.headers());
    let mut broadcast_receiver = state.notification_sender.subscribe();
    let (session_id, mut session_receiver) = state.session_manager.create_session();
    let session_cleanup = SessionCleanup {
        session_manager: state.session_manager.clone(),
        session_id: session_id.clone(),
    };

    // Use a relative endpoint URL with a session ID so the server can route
    // JSON-RPC responses back to the exact SSE connection that sent the request.
    let endpoint = format!("/message?sessionId={}", session_id);

    let stream: std::pin::Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> = Box::pin(async_stream::stream! {
        // Keep the cleanup guard alive for the lifetime of the stream.
        let _cleanup = session_cleanup;

        yield Ok(Event::default().event("endpoint").data(endpoint));

        loop {
            tokio::select! {
                biased;
                notification = session_receiver.recv() => {
                    match notification {
                        Ok(payload) => {
                            yield Ok(Event::default().event("message").data(payload));
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    }
                }
                notification = broadcast_receiver.recv() => {
                    match notification {
                        Ok(payload) => {
                            yield Ok(Event::default().event("message").data(payload));
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                    yield Ok(Event::default().comment("keep-alive"));
                }
            }
        }
    });

    let sse = Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(tokio::time::Duration::from_secs(30))
            .text("keep-alive"),
    );

    let mut response = sse.into_response();
    let headers = response.headers_mut();
    if let Ok(value) = "no-store".parse() {
        headers.insert(axum::http::header::CACHE_CONTROL, value);
    }
    if let Ok(value) = "keep-alive".parse() {
        headers.insert(axum::http::header::CONNECTION, value);
    }
    if let Ok(value) = "no".parse() {
        headers.insert("X-Accel-Buffering", value);
    }
    response
}

/// Removes a session from the manager when the SSE stream ends.
struct SessionCleanup {
    session_manager: SessionManager,
    session_id: String,
}

impl Drop for SessionCleanup {
    fn drop(&mut self) {
        self.session_manager.remove_session(&self.session_id);
    }
}
