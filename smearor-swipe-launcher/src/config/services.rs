use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use smearor_model_plugin::PluginEntry;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use std::collections::HashMap;
use tracing::debug;
use tracing::trace;

/// Configuration for the MCP server section in `services.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct McpConfig {
    /// Address to bind the HTTP server to. Default: `127.0.0.1`.
    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    /// TCP port to listen on. Default: `8765`.
    #[serde(default = "default_port")]
    pub port: u16,

    /// Optional bearer token required for all HTTP requests.
    pub auth_token: Option<String>,
}

fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8765
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            port: default_port(),
            auth_token: None,
        }
    }
}

/// Configuration for the embedded web server section in `services.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct WebConfig {
    /// Whether the web server is enabled. Default: `false`.
    #[serde(default)]
    pub enabled: bool,

    /// TCP port to listen on. Default: `8080`.
    #[serde(default = "default_web_port")]
    pub port: u16,

    /// Address to bind to. Default: `127.0.0.1`.
    #[serde(default = "default_web_bind")]
    pub bind_address: String,

    /// Optional bearer token required for all HTTP requests.
    /// If set, clients must send `Authorization: Bearer <token>`.
    pub auth_token: Option<String>,

    /// Allowed CORS origins. If empty, defaults to localhost origins.
    /// Use `["*"]` to allow all origins (not recommended for production).
    #[serde(default)]
    pub allowed_origins: Vec<String>,
}

fn default_web_port() -> u16 {
    8080
}

fn default_web_bind() -> String {
    "127.0.0.1".to_string()
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_web_port(),
            bind_address: default_web_bind(),
            auth_token: None,
            allowed_origins: Vec::new(),
        }
    }
}

/// Configuration for shared background services.
///
/// Loaded once by `LauncherHost` and shared across all launcher instances.
/// D-Bus services (e.g. notifications, MPRIS) should only be registered
/// once per process and broadcast to all instances via the central broker.
#[derive(Debug, Clone, Deserialize)]
pub struct ServicesConfig {
    /// Services to load
    #[serde(default)]
    pub services: Vec<PluginEntry>,

    /// MCP server configuration
    #[serde(default)]
    pub mcp: McpConfig,

    /// Web server configuration
    #[serde(default)]
    pub web: WebConfig,

    /// Per-service configuration keyed by service ID
    #[serde(flatten)]
    pub entries: HashMap<String, Value>,
}

impl Default for ServicesConfig {
    fn default() -> Self {
        Self {
            services: Vec::new(),
            mcp: McpConfig::default(),
            web: WebConfig::default(),
            entries: HashMap::new(),
        }
    }
}

impl ServicesConfig {
    /// Get plugin configuration by service ID
    pub fn get_service_config(&self, service_id: &str) -> Option<&Value> {
        self.entries.get(service_id)
    }

    /// Get plugin config for plugin API (legacy method for compatibility)
    pub fn plugin_config(&self, id: &str) -> PluginConfig {
        let config = self.get_service_config(id).cloned().unwrap_or_else(|| {
            trace!("No config found for service {id}, using empty config");
            json!({})
        });
        PluginConfig { config }
    }
}
