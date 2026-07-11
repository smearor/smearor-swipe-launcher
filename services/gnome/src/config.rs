/// Configuration for the GNOME workspace tracking service.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct GnomeWorkspaceServiceConfig {
    /// Enable workspace change event tracking and broadcasting.
    #[serde(default)]
    pub enable_workspace_tracking: bool,
    /// Polling interval in milliseconds for querying the active workspace.
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
    /// Enable monitor hotplug detection via MonitorsChanged signal.
    #[serde(default = "default_enable_monitor_events")]
    pub enable_monitor_events: bool,
    /// Enable workspace creation/deletion detection.
    #[serde(default = "default_enable_workspace_lifecycle")]
    pub enable_workspace_lifecycle: bool,
}

fn default_poll_interval_ms() -> u64 {
    500
}

fn default_enable_monitor_events() -> bool {
    true
}

fn default_enable_workspace_lifecycle() -> bool {
    true
}

impl GnomeWorkspaceServiceConfig {
    /// Parses the service configuration from a JSON value.
    pub fn parse(config_json: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config_json.clone())
    }
}
