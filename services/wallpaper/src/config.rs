use serde::Deserialize;
use serde::Serialize;

use smearor_wallpaper_model::WallpaperTheme;

/// Configuration for the wallpaper service.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct WallpaperServiceConfig {
    /// List of all configured wallpaper themes.
    pub themes: Vec<WallpaperTheme>,
    /// Name of the default theme that the service starts with.
    pub default_theme: String,
    /// Path to the configuration file where themes are persisted.
    pub config_path: String,
    /// Whether to automatically start the default theme on service initialization.
    pub auto_start: bool,
    /// Grace period in milliseconds before sending SIGKILL after SIGTERM.
    pub kill_grace_period_ms: u64,
    /// Path to the `mpvpaper` executable. If not set, resolved via `which`.
    #[allow(dead_code)]
    pub mpvpaper_path: Option<String>,
    /// Path to the `smearor-wrot` executable. If not set, resolved via `which`.
    #[allow(dead_code)]
    pub smearor_wrot_path: Option<String>,
    /// Specific Wayland display to use for wallpaper processes (e.g. "wayland-1").
    /// If not set, inherits WAYLAND_DISPLAY from the launcher process environment.
    pub wayland_display: Option<String>,
}

impl Default for WallpaperServiceConfig {
    fn default() -> Self {
        Self {
            themes: Vec::new(),
            default_theme: String::new(),
            config_path: String::from("wallpaper.toml"),
            auto_start: false,
            kill_grace_period_ms: default_kill_grace_period_ms(),
            mpvpaper_path: None,
            smearor_wrot_path: None,
            wayland_display: None,
        }
    }
}

fn default_kill_grace_period_ms() -> u64 {
    3000
}
