use serde::Deserialize;
use serde::Serialize;
use smearor_personalization_model::ColorScheme;

/// Color scheme mode for a theme.
/// Determines how the theme resolves CSS files and reacts to system color scheme changes.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    /// Follow system color scheme (default). Resolves to Dark or Light based on
    /// the personalization service's ColorScheme. Uses `css_files_dark` when
    /// resolved to Dark, `css_files_light` when resolved to Light.
    #[default]
    System,
    /// Fixed dark mode. Uses `css_files_dark`.
    Dark,
    /// Fixed light mode. Uses `css_files_light` (falls back to `css_files_dark` if empty).
    Light,
}

impl std::str::FromStr for ThemeMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "System" | "system" => Ok(ThemeMode::System),
            "Dark" | "dark" => Ok(ThemeMode::Dark),
            "Light" | "light" => Ok(ThemeMode::Light),
            _ => Err(format!("Unknown theme mode: {s}")),
        }
    }
}

impl ThemeMode {
    /// Resolves the effective mode given the current system color scheme.
    /// System mode resolves to Dark or Light based on the personalization status.
    /// Dark and Light modes return themselves unchanged.
    pub fn resolve(self, system_scheme: ColorScheme) -> Self {
        match self {
            ThemeMode::System => match system_scheme {
                ColorScheme::Dark => ThemeMode::Dark,
                ColorScheme::Light => ThemeMode::Light,
                ColorScheme::System => ThemeMode::Dark,
            },
            ThemeMode::Dark => ThemeMode::Dark,
            ThemeMode::Light => ThemeMode::Light,
        }
    }
}
