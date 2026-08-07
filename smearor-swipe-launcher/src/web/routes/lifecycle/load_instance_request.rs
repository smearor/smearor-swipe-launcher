use serde::Deserialize;

/// Request body for `POST /api/instances` (load a new instance).
#[derive(Deserialize)]
pub struct LoadInstanceRequest {
    /// The unique instance identifier.
    pub instance_id: String,
    /// Filesystem path to the instance configuration file.
    pub config_path: String,
    /// Instance type string (e.g. `"gtk"`, `"web"`). Defaults to `"gtk"`.
    #[serde(default = "default_instance_type")]
    pub instance_type: String,
    /// Whether the instance should survive restarts. Defaults to `false`.
    #[serde(default)]
    pub persist: bool,
}

fn default_instance_type() -> String {
    "gtk".to_string()
}
