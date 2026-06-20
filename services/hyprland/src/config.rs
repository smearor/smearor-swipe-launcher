/// Configuration for the Hyprland service.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct HyprlandServiceConfig {
    /// Optional path override for the Hyprland socket.
    pub socket_path: Option<String>,
}

impl HyprlandServiceConfig {
    /// Parses the service configuration from a JSON value.
    pub fn parse(config_json: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config_json.clone())
    }
}
