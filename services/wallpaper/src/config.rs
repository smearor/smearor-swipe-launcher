use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::warn;

use smearor_wallpaper_model::WallpaperTheme;

/// Configuration for the wallpaper service.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct WallpaperServiceConfig {
    /// List of all configured wallpaper themes.
    /// Loaded from `config_path` (wallpaper.toml) at startup, not from services.toml.
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

/// Loads themes from the wallpaper configuration file (e.g. `wallpaper.toml`).
/// Returns an empty vector if the file cannot be read or parsed.
pub fn load_themes(config_path: &str) -> Vec<WallpaperTheme> {
    let path = std::path::Path::new(config_path);
    match std::fs::read_to_string(path) {
        Ok(content) => {
            #[derive(Deserialize)]
            struct ThemesFile {
                themes: Vec<WallpaperTheme>,
            }
            match toml::from_str::<ThemesFile>(&content) {
                Ok(file) => {
                    debug!("Wallpaper config: loaded {} theme(s) from {}", file.themes.len(), config_path);
                    file.themes
                }
                Err(e) => {
                    warn!("Wallpaper config: failed to parse {}: {}", config_path, e);
                    Vec::new()
                }
            }
        }
        Err(e) => {
            warn!("Wallpaper config: failed to read {}: {}", config_path, e);
            Vec::new()
        }
    }
}
