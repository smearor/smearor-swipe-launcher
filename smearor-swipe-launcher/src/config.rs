use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct LauncherConfig {
    pub launcher: LauncherSettings,
    pub left_area: AreaConfig,
    pub scroll_band: AreaConfig,
    pub right_area: AreaConfig,
    #[serde(flatten)]
    pub plugins: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct LauncherSettings {
    pub rotation: u32,
}

#[derive(Debug, Deserialize)]
pub struct AreaConfig {
    pub plugins: Vec<PluginEntry>,
}

#[derive(Debug, Deserialize)]
pub struct PluginEntry {
    pub id: String,
    pub path: String,
}
