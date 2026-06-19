use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use smearor_model_plugin::PluginEntry;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use std::collections::HashMap;
use tracing::warn;

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
