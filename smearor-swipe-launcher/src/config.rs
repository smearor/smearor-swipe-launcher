use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_wrot_rotation::SmearorRotation;
use std::collections::HashMap;
use tracing::warn;

#[derive(Debug, Clone, Deserialize)]
pub struct LauncherConfig {
    pub launcher: LauncherSettings,
    pub left_area: AreaConfig,
    pub scroll_band: AreaConfig,
    pub right_area: AreaConfig,
    #[serde(default)]
    pub services: Vec<PluginEntry>,
    #[serde(flatten)]
    pub plugins: HashMap<String, Value>,
}

impl LauncherConfig {
    pub fn plugin_config(&self, id: &str) -> PluginConfig {
        let config = self.plugins.get(id).cloned().unwrap_or_else(|| {
            warn!("No config found for plugin {id}, using empty config");
            json!({})
        });
        PluginConfig { config }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LauncherSettings {
    #[serde(default = "SmearorRotation::default")]
    pub rotation: SmearorRotation,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AreaConfig {
    pub plugins: Vec<PluginEntry>,
}

impl AreaConfig {
    pub fn plugin_ids(&self) -> Vec<String> {
        self.plugins.iter().map(|p| p.id.clone()).collect()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginEntry {
    pub id: String,
    pub path: String,
}
