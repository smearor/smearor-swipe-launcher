use serde::Deserialize;
use serde::Serialize;

use crate::ThemeColors;
use crate::ThemeMode;

/// A theme definition with metadata, CSS files, theme colors, and optional wallpaper coupling.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Theme {
    /// Human-readable name of the theme (e.g. "default", "Halloween").
    pub name: String,
    /// Description of the theme.
    #[serde(default)]
    pub description: String,
    /// Nerd Font icon name shown in the widget tile (e.g. "nf-md-palette").
    #[serde(default)]
    pub preview_icon: String,
    /// Optional path to a preview image shown in the widget tile.
    /// When set, the widget displays this image instead of the Nerd Font icon.
    #[serde(default)]
    pub preview_image_path: String,
    /// Color scheme mode: Dark, Light, or System.
    /// System mode resolves based on the personalization service's ColorScheme.
    #[serde(default)]
    pub mode: ThemeMode,
    /// CSS file paths applied when the effective mode is Dark.
    /// Used for Dark mode and System mode (when resolved to Dark).
    /// Multiple files may be provided; all are loaded as separate CssProviders.
    #[serde(default)]
    pub css_files_dark: Vec<String>,
    /// CSS file paths applied when the effective mode is Light.
    /// Used for Light mode and System mode (when resolved to Light).
    /// Multiple files may be provided; all are loaded as separate CssProviders.
    /// If empty, `css_files_dark` is used as fallback for Light mode.
    #[serde(default)]
    pub css_files_light: Vec<String>,
    /// Theme colors for Dark and Light modes (5 hex strings each).
    /// Defaults to the official Smearor design palette for both modes.
    #[serde(default)]
    pub colors: ThemeColors,
    /// Optional wallpaper theme name to couple with this theme.
    /// When set, applying this theme also selects and starts the named wallpaper theme.
    #[serde(default)]
    pub wallpaper_theme: Option<String>,
}
