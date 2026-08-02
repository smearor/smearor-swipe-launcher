use serde::Deserialize;

/// Auto-dimming configuration shared across MacroPad services.
#[derive(Debug, Clone, Deserialize)]
pub struct DimmingConfig {
    /// Whether auto-dimming is enabled for all devices by default.
    #[serde(default = "default_auto_dimming_enabled")]
    pub auto_dimming_enabled: bool,
    /// Idle timeout in milliseconds before dimming starts.
    #[serde(default = "default_auto_dim_timeout_ms")]
    pub auto_dim_timeout_ms: u64,
    /// Dimmed brightness level (0-100).
    #[serde(default = "default_auto_dim_brightness")]
    pub auto_dim_brightness: u8,
    /// Fade step interval in milliseconds.
    #[serde(default = "default_auto_dim_fade_step_ms")]
    pub auto_dim_fade_step_ms: u64,
    /// Brightness change per fade step when dimming down (in percent points).
    #[serde(default = "default_auto_dim_fade_step_percent")]
    pub auto_dim_fade_step_percent: u8,
    /// Brightness change per fade step when restoring brightness (in percent points).
    #[serde(default = "default_auto_dim_fade_up_step_percent")]
    pub auto_dim_fade_up_step_percent: u8,
}

impl Default for DimmingConfig {
    fn default() -> Self {
        Self {
            auto_dimming_enabled: default_auto_dimming_enabled(),
            auto_dim_timeout_ms: default_auto_dim_timeout_ms(),
            auto_dim_brightness: default_auto_dim_brightness(),
            auto_dim_fade_step_ms: default_auto_dim_fade_step_ms(),
            auto_dim_fade_step_percent: default_auto_dim_fade_step_percent(),
            auto_dim_fade_up_step_percent: default_auto_dim_fade_up_step_percent(),
        }
    }
}

/// Per-device dimming configuration override. All fields are optional.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DimmingConfigOverride {
    /// Whether auto-dimming is enabled for this device. If omitted, uses service default.
    pub auto_dimming_enabled: Option<bool>,
    /// Idle timeout in milliseconds. If omitted, uses service default.
    pub auto_dim_timeout_ms: Option<u64>,
    /// Dimmed brightness level (0-100). If omitted, uses service default.
    pub auto_dim_brightness: Option<u8>,
    /// Fade step interval in milliseconds. If omitted, uses service default.
    pub auto_dim_fade_step_ms: Option<u64>,
    /// Brightness change per fade step when dimming down. If omitted, uses service default.
    pub auto_dim_fade_step_percent: Option<u8>,
    /// Brightness change per fade step when restoring brightness. If omitted, uses service default.
    pub auto_dim_fade_up_step_percent: Option<u8>,
}

impl DimmingConfigOverride {
    /// Merge this override on top of a global `DimmingConfig`, producing a resolved config.
    pub fn merge(&self, global: &DimmingConfig) -> DimmingConfig {
        DimmingConfig {
            auto_dimming_enabled: self.auto_dimming_enabled.unwrap_or(global.auto_dimming_enabled),
            auto_dim_timeout_ms: self.auto_dim_timeout_ms.unwrap_or(global.auto_dim_timeout_ms),
            auto_dim_brightness: self.auto_dim_brightness.unwrap_or(global.auto_dim_brightness),
            auto_dim_fade_step_ms: self.auto_dim_fade_step_ms.unwrap_or(global.auto_dim_fade_step_ms),
            auto_dim_fade_step_percent: self.auto_dim_fade_step_percent.unwrap_or(global.auto_dim_fade_step_percent),
            auto_dim_fade_up_step_percent: self.auto_dim_fade_up_step_percent.unwrap_or(global.auto_dim_fade_up_step_percent),
        }
    }
}

fn default_auto_dimming_enabled() -> bool {
    true
}

fn default_auto_dim_timeout_ms() -> u64 {
    30000
}

fn default_auto_dim_brightness() -> u8 {
    5
}

fn default_auto_dim_fade_step_ms() -> u64 {
    50
}

fn default_auto_dim_fade_step_percent() -> u8 {
    5
}

fn default_auto_dim_fade_up_step_percent() -> u8 {
    10
}
