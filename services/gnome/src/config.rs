/// Configuration for the GNOME workspace tracking service.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct GnomeWorkspaceServiceConfig {
    /// Enable workspace change event tracking and broadcasting.
    #[serde(default)]
    pub enable_workspace_tracking: bool,
    /// Polling interval in milliseconds for querying the active workspace.
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
}

fn default_poll_interval_ms() -> u64 {
    500
}

impl GnomeWorkspaceServiceConfig {
    /// Parses the service configuration from a JSON value.
    pub fn parse(config_json: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config_json.clone())
    }
}
