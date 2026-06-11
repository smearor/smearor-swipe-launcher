use crate::SwipeLauncherArguments;
use crate::config::area::AreaConfig;
use crate::config::layer::LayerConfigFile;
use crate::config::merge::MergeWithArguments;
use crate::config::plugin::PluginEntry;
use crate::config::rotation::RotationConfigFile;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use std::collections::HashMap;
use tracing::warn;

#[derive(Debug, Clone, Deserialize)]
pub struct SwipeLauncherConfig {
    pub launcher: SwipeLauncherSettings,
    pub left_area: AreaConfig,
    pub scroll_band: AreaConfig,
    pub right_area: AreaConfig,
    #[serde(default)]
    pub services: Vec<PluginEntry>,
    #[serde(flatten)]
    pub plugins: HashMap<String, Value>,
}

impl SwipeLauncherConfig {
    pub fn plugin_config(&self, id: &str) -> PluginConfig {
        let config = self.plugins.get(id).cloned().unwrap_or_else(|| {
            warn!("No config found for plugin {id}, using empty config");
            json!({})
        });
        PluginConfig { config }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SwipeLauncherSettings {
    /// Configuration for the layer
    #[serde(default, flatten)]
    pub layer: LayerConfigFile,

    /// Configuration for the rotation
    #[serde(default, flatten)]
    pub rotation: RotationConfigFile,
}

impl MergeWithArguments<SwipeLauncherArguments> for SwipeLauncherSettings {
    fn merge_with_arguments(self, args: &SwipeLauncherArguments) -> Self {
        let mut config = self;
        config.rotation = config.rotation.merge_with_arguments(&args.rotation);
        config.layer = config.layer.merge_with_arguments(&args.layer);
        config
    }
}
