use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use tracing::debug;
use tracing::warn;

use smearor_wallpaper_model::WallpaperTheme;
use smearor_wallpaper_model::WallpaperThemeConfig;

/// Expands `~` in all path-like fields of a wallpaper theme.
/// This allows users to write `~/Videos/Backgrounds` instead of `/home/user/Videos/Backgrounds`.
fn expand_theme_paths(theme: &mut WallpaperTheme) {
    theme.preview_image_path = shellexpand::tilde(&theme.preview_image_path).into_owned();
    match &mut theme.config {
        WallpaperThemeConfig::Video(config) => {
            config.directory = shellexpand::tilde(&config.directory).into_owned();
        }
        WallpaperThemeConfig::Image(config) => {
            config.directory = shellexpand::tilde(&config.directory).into_owned();
        }
        WallpaperThemeConfig::Application(config) => {
            config.command = shellexpand::tilde(&config.command).into_owned();
            for arg in &mut config.arguments {
                *arg = shellexpand::tilde(arg).into_owned();
            }
        }
    }
}

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
    ///
    /// If empty, the host resolves the path via `ConfigDiscoveryService`.
    /// Falls back to discovery within the service if the host does not inject a path.
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

impl WallpaperServiceConfig {
    pub fn load_or_discover_themes(&self) -> Vec<WallpaperTheme> {
        if self.config_path.is_empty() {
            if let Some(discovered) = self.discover_wallpaper_config() {
                return self.load_themes_with_config(&discovered);
            }
            return Vec::new();
        }
        let path = Path::new(&self.config_path);
        if path.is_file() {
            self.load_themes_with_config(path)
        } else if let Some(discovered) = self.discover_wallpaper_config() {
            self.load_themes_with_config(&discovered)
        } else {
            Vec::new()
        }
    }

    /// Loads themes from the wallpaper configuration file (e.g. `wallpaper.toml`).
    /// Returns an empty vector if the file cannot be read or parsed.
    pub fn load_themes(&self) -> Vec<WallpaperTheme> {
        self.load_themes_with_config(Path::new(&self.config_path))
    }

    pub fn load_themes_with_config(&self, path: &Path) -> Vec<WallpaperTheme> {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                #[derive(Deserialize)]
                struct ThemesFile {
                    themes: Vec<WallpaperTheme>,
                }
                match toml::from_str::<ThemesFile>(&content) {
                    Ok(mut file) => {
                        for theme in &mut file.themes {
                            expand_theme_paths(theme);
                        }
                        debug!("Wallpaper config: loaded {} theme(s) from {}", file.themes.len(), &self.config_path);
                        file.themes
                    }
                    Err(e) => {
                        warn!("Wallpaper config: failed to parse {}: {}", &self.config_path, e);
                        Vec::new()
                    }
                }
            }
            Err(e) => {
                warn!("Wallpaper config: failed to read {}: {}", &self.config_path, e);
                Vec::new()
            }
        }
    }

    /// Discovers the wallpaper config file (`wallpaper.toml`) using fallback locations.
    ///
    /// Checks the working directory first, then `~/.config/smearor/services/wallpaper.toml`,
    /// then `/usr/share/smearor/services/wallpaper.toml` (system default).
    /// Returns `None` if no file is found.
    fn discover_wallpaper_config(&self) -> Option<PathBuf> {
        if let Ok(cwd) = std::env::current_dir() {
            let path = cwd.join("wallpaper.toml");
            if path.is_file() {
                debug!("Discovered wallpaper config in working directory: {}", path.display());
                return Some(path);
            }
            let configs_path = cwd.join("configs").join("services").join("wallpaper.toml");
            if configs_path.is_file() {
                debug!("Discovered wallpaper config in configs/services: {}", configs_path.display());
                return Some(configs_path);
            }
        }

        if let Some(config_dir) = dirs::config_dir() {
            let path = config_dir.join("smearor").join("services").join("wallpaper.toml");
            if path.is_file() {
                debug!("Discovered wallpaper config in XDG config directory: {}", path.display());
                return Some(path);
            }
        }

        let system_path = Path::new("/usr/share/smearor/services/wallpaper.toml");
        if system_path.is_file() {
            debug!("Discovered wallpaper config in system directory: {}", system_path.display());
            return Some(system_path.to_path_buf());
        }

        debug!("No wallpaper config found, starting with empty themes list");
        None
    }
}

impl Default for WallpaperServiceConfig {
    fn default() -> Self {
        Self {
            themes: Vec::new(),
            default_theme: String::new(),
            config_path: String::new(),
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
