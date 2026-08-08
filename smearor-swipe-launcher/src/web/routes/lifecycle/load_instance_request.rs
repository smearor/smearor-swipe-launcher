use serde::Deserialize;
use smearor_mcp_server::InstanceTypeParam;

/// Request body for `POST /api/instances` (load a new instance).
#[derive(Deserialize)]
pub struct LoadInstanceRequest {
    /// The unique instance identifier.
    pub instance_id: String,
    /// Filesystem path to the instance configuration file.
    pub config_path: String,
    /// Instance type: `"gtk"`, `"headless"`, or `"web"`. Defaults to `"gtk"`.
    #[serde(default)]
    pub instance_type: InstanceTypeParam,
    /// Whether the instance should survive restarts. Defaults to `false`.
    #[serde(default)]
    pub persist: bool,
}
