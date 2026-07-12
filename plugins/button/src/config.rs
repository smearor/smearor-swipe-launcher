use serde::Deserialize;
use serde_json::Value;

pub const DEFAULT_WIDTH: i32 = 100;

pub const DEFAULT_ICON_SIZE: i32 = 36;

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
    /// Target instance for single-click message
    #[serde(default)]
    pub click_instance: Option<String>,
    /// Message topic for long-press
    #[serde(default)]
    pub longpress_topic: Option<String>,
    /// Message payload for long-press (JSON/TOML)
    #[serde(default)]
    pub longpress_payload: Option<Value>,
    /// Target instance for long-press message
    #[serde(default)]
    pub longpress_instance: Option<String>,
    /// Message topic for swipe-up gesture
    #[serde(default)]
    pub swipe_up_topic: Option<String>,
    /// Message payload for swipe-up gesture (JSON/TOML)
    #[serde(default)]
    pub swipe_up_payload: Option<Value>,
    /// Target instance for swipe-up message
    #[serde(default)]
    pub swipe_up_instance: Option<String>,
    /// Message topic for swipe-down gesture
    #[serde(default)]
    pub swipe_down_topic: Option<String>,
    /// Message payload for swipe-down gesture (JSON/TOML)
    #[serde(default)]
    pub swipe_down_payload: Option<Value>,
    /// Target instance for swipe-down message
    #[serde(default)]
    pub swipe_down_instance: Option<String>,
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
    /// Spacing between child widgets inside the button
    #[serde(default)]
    pub spacing: i32,
    /// Additional CSS classes for styling
    #[serde(default)]
    pub css_classes: Vec<String>,
    /// Topic whose messages control the label text.
    #[serde(default)]
    pub label_topic: Option<String>,
    /// Format string for the label display (JSON values via serde_json).
    #[serde(default)]
    pub label_format: Option<String>,
    /// Fallback text when the topic has not yet delivered a message.
    #[serde(default)]
    pub label_fallback: Option<String>,
    /// Topic whose messages update the internal state (JSON)
    #[serde(default)]
    pub state_topic: Option<String>,
    /// Initial one-shot request topic (sent on widget construction)
    #[serde(default)]
    pub init_topic: Option<String>,
    /// Initial one-shot request payload (JSON/TOML)
    #[serde(default)]
    pub init_payload: Option<Value>,
    /// Target instance for the initial one-shot request
    #[serde(default)]
    pub init_instance: Option<String>,
    /// Icon expression evaluated against the internal state.
    /// Supports static icon names and conditional expressions like "{ison?nf-md-fan:nf-md-fan-off}".
    #[serde(default)]
    pub state_icon: Option<String>,
    /// CSS class added when the internal state is truthy, removed when falsy.
    #[serde(default)]
    pub state_css_class: Option<String>,
    /// Label format string evaluated against the internal state.
    #[serde(default)]
    pub state_label: Option<String>,
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
