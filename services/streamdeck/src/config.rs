use serde::Deserialize;
use smearor_model_macropad::DimmingConfig;
use smearor_model_macropad::DimmingConfigOverride;

/// Configuration for the Stream Deck service.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamDeckConfig {
    /// Polling interval for reading button states in milliseconds.
    #[serde(default = "default_poll_interval")]
    pub poll_interval_ms: u64,
    /// Initial brightness (0-100).
    #[serde(default = "default_brightness")]
    pub brightness: u8,
    /// Whether to enable MCP tool registration for this service.
    #[serde(default = "default_mcp_enabled")]
    pub mcp_enabled: bool,
    /// Auto-dimming configuration.
    #[serde(flatten)]
    pub dimming: DimmingConfig,
    /// Per-device configuration overrides.
    #[serde(default)]
    pub device_overrides: Vec<DeviceOverride>,
}

impl StreamDeckConfig {
    pub fn parse(config: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config.clone())
    }
}

/// Per-device configuration override.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceOverride {
    /// Device serial number.
    pub serial: String,
    /// Initial brightness (0-100). If omitted, uses service default.
    pub brightness: Option<u8>,
    /// Per-device dimming configuration override.
    #[serde(flatten)]
    pub dimming: DimmingConfigOverride,
}

fn default_poll_interval() -> u64 {
    50
}

fn default_brightness() -> u8 {
    50
}

fn default_mcp_enabled() -> bool {
    true
}
