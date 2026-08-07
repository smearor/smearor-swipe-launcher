use crate::web::config::WebServerConfig;
use crate::web::routes::api_list_instances;
use crate::web::routes::api_load_instance;
use crate::web::routes::api_reload_instance;
use crate::web::routes::api_start_instance;
use crate::web::routes::api_stop_instance;
use crate::web::routes::api_unload_instance;
use crate::web::routes::handle_action;
use crate::web::routes::handle_websocket;
use crate::web::routes::list_web_instances;
use crate::web::routes::serve_instance_page;
use crate::web::routes::serve_static_css;
use crate::web::routes::serve_static_js;
use crate::web::routes::serve_static_nerdfont_css;
use crate::web::routes::serve_static_nerdfont_woff2;
use crate::web::state::WebAppState;
use crate::web::template::TemplateEngine;
use crate::web::ws_manager::WebSocketManager;
use axum::Router;
use axum::response::IntoResponse;
use axum::routing::delete;
use axum::routing::get;
use axum::routing::post;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use tracing::error;

/// Embedded HTTP server that serves web launcher instances.
///
/// Runs on a tokio task alongside the main GTK application. Serves
/// multiple web instances at `/instances/{id}/`.
pub struct WebServer {
    config: WebServerConfig,
    state: Arc<WebAppState>,
    ws_manager: Arc<WebSocketManager>,
}

impl WebServer {
    pub fn new(
        config: WebServerConfig,
        instances: Arc<Mutex<HashMap<String, crate::instance::LauncherInstance>>>,
        broker_sender: UnboundedSender<smearor_swipe_launcher_plugin_api::FfiEnvelope>,
        mcp_command_sender: async_channel::Sender<smearor_mcp_server::McpCommand>,
    ) -> Self {
        let ws_manager = Arc::new(WebSocketManager::new());
        let state = Arc::new(WebAppState {
            instances,
            broker_sender,
            template_engine: TemplateEngine::new(),
            ws_manager: ws_manager.clone(),
            mcp_command_sender,
        });

        Self { config, state, ws_manager }
    }

    /// Get a reference to the WebSocket manager for broker forwarding.
    pub fn ws_manager(&self) -> &Arc<WebSocketManager> {
        &self.ws_manager
    }

    /// Get the web server configuration.
    pub fn config(&self) -> &WebServerConfig {
        &self.config
    }

    /// Register a web instance for WebSocket updates.
    pub fn register_instance(&self, instance_id: &str) {
        self.ws_manager.register_instance(instance_id);
    }

    /// Unregister a web instance and close all its WebSocket connections.
    pub fn unregister_instance(&self, instance_id: &str) {
        self.ws_manager.unregister_instance(instance_id);
    }

    /// Start the web server on a tokio task.
    pub fn start(&self) {
        if !self.config.enabled {
            debug!("Web server is disabled, not starting");
            return;
        }

        let app = self
            .build_router()
            .with_state(self.state.clone())
            .layer(axum::middleware::from_fn_with_state(self.config.auth_token.clone(), auth_middleware))
            .layer(build_cors_layer(&self.config.allowed_origins));

        let addr = format!("{}:{}", self.config.bind_address, self.config.port);

        debug!("Starting web server on {}", addr);

        tokio::spawn(async move {
            let listener = match tokio::net::TcpListener::bind(&addr).await {
                Ok(listener) => listener,
                Err(e) => {
                    error!("Failed to bind web server to {}: {}", addr, e);
                    return;
                }
            };

            debug!("Web server listening on {}", addr);

            axum::serve(listener, app).await.unwrap_or_else(|e| error!("Web server error: {}", e));
        });
    }

    fn build_router(&self) -> Router<Arc<WebAppState>> {
        Router::new()
            .route("/instances", get(list_web_instances))
            .route("/instances/{id}", get(serve_instance_page))
            .route("/instances/{id}/ws", get(handle_websocket))
            .route("/instances/{id}/{plugin_id}/{action}", post(handle_action))
            .route("/api/instances", get(api_list_instances).post(api_load_instance))
            .route("/api/instances/{id}/start", post(api_start_instance))
            .route("/api/instances/{id}/stop", post(api_stop_instance))
            .route("/api/instances/{id}/reload", post(api_reload_instance))
            .route("/api/instances/{id}", delete(api_unload_instance))
            .route("/static/style.css", get(serve_static_css))
            .route("/static/app.js", get(serve_static_js))
            .route("/static/nerdfont.css", get(serve_static_nerdfont_css))
            .route("/static/nerdfont.woff2", get(serve_static_nerdfont_woff2))
    }
}

/// Bearer token authentication middleware.
///
/// If `auth_token` is `None`, all requests are allowed.
/// If set, requests must include `Authorization: Bearer <token>`.
/// WebSocket connections pass the token via the `Sec-WebSocket-Protocol` header
/// or a `token` query parameter.
async fn auth_middleware(
    axum::extract::State(expected_token): axum::extract::State<Option<String>>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let Some(ref expected) = expected_token else {
        return next.run(request).await;
    };

    // Check Authorization header
    if let Some(auth_header) = request.headers().get(axum::http::header::AUTHORIZATION) {
        if let Ok(auth_value) = auth_header.to_str() {
            if auth_value == format!("Bearer {}", expected) {
                return next.run(request).await;
            }
        }
    }

    // Check token query parameter (for WebSocket connections)
    if let Some(query) = request.uri().query() {
        for pair in query.split('&') {
            if let Some(token) = pair.strip_prefix("token=") {
                if token == expected {
                    return next.run(request).await;
                }
            }
        }
    }

    debug!("Web server: rejected unauthenticated request");
    (axum::http::StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
}

/// Build a CORS layer from the configured allowed origins.
///
/// If no origins are configured, defaults to localhost-only.
/// If `["*"]` is set, allows all origins.
fn build_cors_layer(allowed_origins: &[String]) -> tower_http::cors::CorsLayer {
    let origins: Vec<axum::http::HeaderValue> = if allowed_origins.is_empty() {
        vec!["http://localhost".parse().unwrap(), "http://127.0.0.1".parse().unwrap()]
    } else if allowed_origins.iter().any(|o| o == "*") {
        return tower_http::cors::CorsLayer::permissive();
    } else {
        allowed_origins.iter().filter_map(|o| o.parse().ok()).collect()
    };

    tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::list(origins))
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([axum::http::header::CONTENT_TYPE])
}
