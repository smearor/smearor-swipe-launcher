use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PluginEntry {
    /// The instance id of the plugin
    pub id: String,

    /// The path to the shared library of the plugin (.so file)
    pub path: String,
}
