use crate::config::plugin::PluginEntry;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AreaConfig {
    pub plugins: Vec<PluginEntry>,
}

impl AreaConfig {
    pub fn plugin_ids(&self) -> Vec<String> {
        self.plugins.iter().map(|p| p.id.clone()).collect()
    }
}
