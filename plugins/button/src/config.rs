use serde::Deserialize;
use serde_json::Value;

pub const DEFAULT_WIDTH: i32 = 100;

pub const DEFAULT_ICON_SIZE: i32 = 48;

/// Configuration for a button widget
#[derive(Debug, Clone, Deserialize)]
pub struct ButtonConfig {
    /// Button label text (hidden if icon_only is true)
    pub text: String,
    /// Button width
    #[serde(default = "default_width")]
    pub width: i32,
    /// Icon name from icon theme
    #[serde(default)]
    pub icon: Option<String>,
    /// Icon size
    #[serde(default = "default_icon_size")]
    pub icon_size: i32,
    /// Tooltip text on hover
    #[serde(default)]
    pub tooltip: Option<String>,
    /// Show only icon without text
    #[serde(default)]
    pub icon_only: bool,
    /// Message topic for single-click
    #[serde(default)]
    pub click_topic: Option<String>,
    /// Message payload for single-click (JSON/TOML)
    #[serde(default)]
    pub click_payload: Option<Value>,
    /// Message topic for long-press
    #[serde(default)]
    pub longpress_topic: Option<String>,
    /// Message payload for long-press (JSON/TOML)
    #[serde(default)]
    pub longpress_payload: Option<Value>,
    /// Keyboard shortcut (e.g., "Ctrl+G", "Alt+F1")
    #[serde(default)]
    pub shortcut: Option<String>,
    /// Whether the button is interactive
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Whether the button is in active state
    #[serde(default)]
    pub active: bool,
    /// Animation type on button press (scale, fade, ripple)
    #[serde(default)]
    pub press_animation: Option<String>,
    /// Additional CSS classes for styling
    #[serde(default)]
    pub css_classes: Vec<String>,
}

impl ButtonConfig {
    pub fn parse(config: &Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config.clone())
    }
}

fn default_enabled() -> bool {
    true
}

fn default_width() -> i32 {
    DEFAULT_WIDTH
}

fn default_icon_size() -> i32 {
    DEFAULT_ICON_SIZE
}
