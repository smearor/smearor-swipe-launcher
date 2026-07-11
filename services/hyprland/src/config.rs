/// Configuration for the Hyprland service.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct HyprlandServiceConfig {
    /// Optional path override for the Hyprland socket.
    pub socket_path: Option<String>,
    /// Enable workspace change event tracking and broadcasting.
    #[serde(default)]
    pub enable_workspace_tracking: bool,
    /// Enable monitor hotplug event broadcasting.
    #[serde(default = "default_enable_monitor_events")]
    pub enable_monitor_events: bool,
    /// Enable workspace creation/deletion event broadcasting.
    #[serde(default = "default_enable_workspace_lifecycle")]
    pub enable_workspace_lifecycle: bool,
}

fn default_enable_monitor_events() -> bool {
    true
}

fn default_enable_workspace_lifecycle() -> bool {
    true
}

impl HyprlandServiceConfig {
    /// Parses the service configuration from a JSON value.
    pub fn parse(config_json: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config_json.clone())
    }
}
