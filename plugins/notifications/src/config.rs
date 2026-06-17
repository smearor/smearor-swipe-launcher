use serde::Deserialize;
use serde_json::Value;

pub const DEFAULT_WIDTH: i32 = 180;

pub const DEFAULT_HEIGHT: i32 = 120;

/// Configuration for the notifications widget.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NotificationWidgetConfig {
    /// Widget width in pixels.
    pub width: i32,
    /// Widget height in pixels.
    pub height: i32,
    /// Maximum number of notifications to display.
    pub max_visible: usize,
    /// Whether to show the Do Not Disturb toggle.
    pub show_dnd_toggle: bool,
    /// Whether to show notification icons.
    pub show_icons: bool,
}

impl NotificationWidgetConfig {
    pub fn parse(config: &Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config.clone())
    }
}

impl Default for NotificationWidgetConfig {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            max_visible: 3,
            show_dnd_toggle: true,
            show_icons: true,
        }
    }
}
