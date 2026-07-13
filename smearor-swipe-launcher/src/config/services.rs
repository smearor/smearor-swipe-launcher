use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use smearor_model_plugin::PluginEntry;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use std::collections::HashMap;
use tracing::warn;

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

    /// Per-service configuration keyed by service ID
    #[serde(flatten)]
    pub entries: HashMap<String, Value>,
}

impl ServicesConfig {
    /// Get plugin configuration by service ID
    pub fn get_service_config(&self, service_id: &str) -> Option<&Value> {
        self.entries.get(service_id)
    }

    /// Get plugin config for plugin API (legacy method for compatibility)
    pub fn plugin_config(&self, id: &str) -> PluginConfig {
        let config = self.get_service_config(id).cloned().unwrap_or_else(|| {
            warn!("No config found for service {id}, using empty config");
            json!({})
        });
        PluginConfig { config }
    }
}
