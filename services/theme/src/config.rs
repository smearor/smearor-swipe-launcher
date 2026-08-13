use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use tracing::debug;
use tracing::warn;

use smearor_theme_model::Theme;

/// Configuration for the theme service.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ThemeServiceConfig {
    /// List of all configured themes.
    /// Loaded from `config_path` (themes.toml) at startup, not from services.toml.
    pub themes: Vec<Theme>,
    /// Name of the default theme to apply on startup.
    pub default_theme: String,
    /// Path to the configuration file where themes are persisted.
    /// If empty, the host resolves the path via config discovery.
    pub config_path: String,
    /// Whether to automatically apply the default theme on service initialization.
    pub auto_apply: bool,
    /// Whether System-mode themes should react to personalization color scheme changes.
    /// When true, the service re-applies CSS when ColorScheme changes.
    pub follow_system_color_scheme: bool,
}

impl ThemeServiceConfig {
    /// Loads themes from the theme configuration file (e.g. `themes.toml`).
    /// Uses config_path if set, otherwise discovers the file.
    /// Returns an empty vector if no file is found or parsed.
    pub fn load_or_discover_themes(&self) -> Vec<Theme> {
        if self.config_path.is_empty() {
            if let Some(discovered) = self.discover_theme_config() {
                return self.load_themes_with_config(&discovered);
            }
            return Vec::new();
        }
        let path = Path::new(&self.config_path);
        if path.is_file() {
            self.load_themes_with_config(path)
        } else if let Some(discovered) = self.discover_theme_config() {
            self.load_themes_with_config(&discovered)
        } else {
            Vec::new()
        }
    }

    /// Loads themes from the given path.
    pub fn load_themes_with_config(&self, path: &Path) -> Vec<Theme> {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                #[derive(Deserialize)]
                struct ThemesFile {
                    themes: Vec<Theme>,
                }
                match toml::from_str::<ThemesFile>(&content) {
                    Ok(file) => {
                        debug!("Theme config: loaded {} theme(s) from {}", file.themes.len(), path.display());
                        file.themes
                    }
                    Err(e) => {
                        warn!("Theme config: failed to parse {}: {}", path.display(), e);
                        Vec::new()
                    }
                }
            }
            Err(e) => {
                warn!("Theme config: failed to read {}: {}", path.display(), e);
                Vec::new()
            }
        }
    }

    /// Discovers the theme config file (`themes.toml`) using fallback locations.
    ///
    /// Checks the working directory first, then `~/.config/smearor/services/themes.toml`,
    /// then `/usr/share/smearor/services/themes.toml` (system default).
    fn discover_theme_config(&self) -> Option<PathBuf> {
        if let Ok(cwd) = std::env::current_dir() {
            let path = cwd.join("themes.toml");
            if path.is_file() {
                debug!("Discovered theme config in working directory: {}", path.display());
                return Some(path);
            }
            let configs_path = cwd.join("configs").join("services").join("themes.toml");
            if configs_path.is_file() {
                debug!("Discovered theme config in configs/services: {}", configs_path.display());
                return Some(configs_path);
            }
        }

        if let Some(config_dir) = dirs::config_dir() {
            let path = config_dir.join("smearor").join("services").join("themes.toml");
            if path.is_file() {
                debug!("Discovered theme config in XDG config directory: {}", path.display());
                return Some(path);
            }
        }

        let system_path = Path::new("/usr/share/smearor/services/themes.toml");
        if system_path.is_file() {
            debug!("Discovered theme config in system directory: {}", system_path.display());
            return Some(system_path.to_path_buf());
        }

        debug!("No theme config found, starting with empty themes list");
        None
    }
}

fn default_true() -> bool {
    true
}

impl Default for ThemeServiceConfig {
    fn default() -> Self {
        Self {
            themes: Vec::new(),
            default_theme: String::new(),
            config_path: String::new(),
            auto_apply: false,
            follow_system_color_scheme: default_true(),
        }
    }
}
