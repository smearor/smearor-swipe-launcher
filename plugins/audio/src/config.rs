use serde::Deserialize;
use serde_json::Value;

pub const DEFAULT_WIDTH: i32 = 100;

pub const DEFAULT_HEIGHT: i32 = 100;

pub const DEFAULT_VOLUME_STEP: f32 = 0.05;

/// Configuration for the audio widget.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AudioWidgetConfig {
    /// Widget width in pixels.
    pub width: i32,
    /// Widget height in pixels.
    pub height: i32,
    /// Volume change step (0.01 to 0.1).
    pub volume_step: f32,
    /// Whether to show the volume bar.
    pub show_volume_bar: bool,
    /// Whether to show the device name label.
    pub show_device_label: bool,
    /// Whether to allow volume over 100%.
    pub allow_overdrive: bool,
}

impl AudioWidgetConfig {
    pub fn parse(config: &Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config.clone())
    }
}

impl Default for AudioWidgetConfig {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            volume_step: DEFAULT_VOLUME_STEP,
            show_volume_bar: true,
            show_device_label: true,
            allow_overdrive: false,
        }
    }
}
