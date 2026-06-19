use serde::Deserialize;
use serde_json::Value;

pub const DEFAULT_WIDTH: i32 = 100;

pub const DEFAULT_HEIGHT: i32 = 100;

pub const DEFAULT_ICON_SIZE: i32 = 36;

/// Configuration for the MPRIS widget.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MprisWidgetConfig {
    /// Widget width in pixels.
    pub width: i32,
    /// Widget height in pixels.
    pub height: i32,
    /// Whether to show the album art.
    pub show_album_art: bool,
    /// Whether to show the progress bar.
    pub show_progress_bar: bool,
    /// Whether to show the player name label.
    pub show_player_label: bool,
    /// List of allowed player bus names (empty = all players).
    pub player_filter: Vec<String>,
    /// Spacing between child widgets inside the MPRIS widget.
    pub spacing: i32,
    /// Maximum width in characters for title and artist labels.
    pub max_width_chars: i32,
    /// Size of the album art icon in pixels.
    pub icon_size: i32,
}

impl MprisWidgetConfig {
    pub fn parse(config: &Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config.clone())
    }
}

impl Default for MprisWidgetConfig {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            show_album_art: true,
            show_progress_bar: true,
            show_player_label: true,
            player_filter: Vec::new(),
            spacing: 0,
            max_width_chars: 24,
            icon_size: DEFAULT_ICON_SIZE,
        }
    }
}
