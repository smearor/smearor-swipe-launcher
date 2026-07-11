/// Configuration for the Wayland workspace tracking service.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct WaylandWorkspaceServiceConfig {
    /// Enable workspace change event tracking and broadcasting.
    #[serde(default)]
    pub enable_workspace_tracking: bool,
}

impl WaylandWorkspaceServiceConfig {
    /// Parses the service configuration from a JSON value.
    pub fn parse(config_json: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config_json.clone())
    }
}
