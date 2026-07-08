use serde::Deserialize;
use serde_json::Value;
use typed_builder::TypedBuilder;

pub const DEFAULT_WIDTH: i32 = 120;
pub const DEFAULT_HEIGHT: i32 = 120;
pub const DEFAULT_PREVIEW_WIDTH: i32 = 100;
pub const DEFAULT_PREVIEW_HEIGHT: i32 = 100;
pub const DEFAULT_FALLBACK_ICON: &str = "nf-md-wallpaper";

/// Configuration for the wallpaper widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct WallpaperWidgetConfig {
    /// The width of the widget in pixels.
    #[builder(default, setter(into))]
    pub(crate) width: Option<i32>,

    /// The height of the widget in pixels.
    #[builder(default, setter(into))]
    pub(crate) height: Option<i32>,

    /// Whether to show the theme name as a label overlay.
    #[builder(default)]
    pub(crate) show_theme_name: bool,

    /// Whether to show the wallpaper type icon.
    #[builder(default)]
    pub(crate) show_type_icon: bool,

    /// Whether to show the running/stopped status indicator.
    #[builder(default)]
    pub(crate) show_status_indicator: bool,

    /// Preview image width in pixels.
    #[builder(default, setter(into))]
    pub(crate) preview_width: Option<i32>,

    /// Preview image height in pixels.
    #[builder(default, setter(into))]
    pub(crate) preview_height: Option<i32>,

    /// Fallback icon when no preview image is available.
    #[builder(default, setter(into))]
    #[serde(default = "default_fallback_icon")]
    pub(crate) fallback_icon: String,

    /// Message topic for single-click action.
    #[serde(default)]
    pub click_topic: Option<String>,

    /// Message payload for single-click action (JSON/TOML).
    #[serde(default)]
    pub click_payload: Option<Value>,

    /// Target instance for single-click message
    #[serde(default)]
    pub click_instance: Option<String>,

    /// Message topic for long-press.
    #[serde(default)]
    pub longpress_topic: Option<String>,

    /// Message payload for long-press (JSON/TOML).
    #[serde(default)]
    pub longpress_payload: Option<Value>,

    /// Target instance for long-press message
    #[serde(default)]
    pub longpress_instance: Option<String>,
}

impl Default for WallpaperWidgetConfig {
    fn default() -> Self {
        Self {
            width: Some(DEFAULT_WIDTH),
            height: Some(DEFAULT_HEIGHT),
            show_theme_name: true,
            show_type_icon: true,
            show_status_indicator: true,
            preview_width: Some(DEFAULT_PREVIEW_WIDTH),
            preview_height: Some(DEFAULT_PREVIEW_HEIGHT),
            fallback_icon: DEFAULT_FALLBACK_ICON.to_string(),
            click_topic: None,
            click_payload: None,
            click_instance: None,
            longpress_topic: None,
            longpress_payload: None,
            longpress_instance: None,
        }
    }
}

fn default_fallback_icon() -> String {
    DEFAULT_FALLBACK_ICON.to_string()
}
