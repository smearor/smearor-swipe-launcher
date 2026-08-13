use serde::Deserialize;
use serde::Serialize;

/// Five theme colors exported as CSS custom properties.
///
/// Each color is a hex string (e.g. "#04e762ff") that the theme service
/// injects as a CSS variable (`--theme-color-1` through `--theme-color-5`)
/// via a generated `:root { ... }` CSS block.
///
/// The default values correspond to the official Smearor design palette
/// defined in `docs/DESIGN.md`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ThemePalette {
    /// Primary color. Exported as `--theme-color-1`.
    /// Default: malachite `#04e762ff`.
    #[serde(default = "default_color_1")]
    pub color_1: String,

    /// Secondary color. Exported as `--theme-color-2`.
    /// Default: selective-yellow `#f5b700ff`.
    #[serde(default = "default_color_2")]
    pub color_2: String,

    /// Tertiary color. Exported as `--theme-color-3`.
    /// Default: celestial-blue `#00a1e4ff`.
    #[serde(default = "default_color_3")]
    pub color_3: String,

    /// Quaternary color. Exported as `--theme-color-4`.
    /// Default: mexican-pink `#dc0073ff`.
    #[serde(default = "default_color_4")]
    pub color_4: String,

    /// Quinary color. Exported as `--theme-color-5`.
    /// Default: chartreuse `#89fc00ff`.
    #[serde(default = "default_color_5")]
    pub color_5: String,
}

impl Default for ThemePalette {
    fn default() -> Self {
        Self {
            color_1: default_color_1(),
            color_2: default_color_2(),
            color_3: default_color_3(),
            color_4: default_color_4(),
            color_5: default_color_5(),
        }
    }
}

fn default_color_1() -> String {
    "#04e762ff".to_string()
}

fn default_color_2() -> String {
    "#f5b700ff".to_string()
}

fn default_color_3() -> String {
    "#00a1e4ff".to_string()
}

fn default_color_4() -> String {
    "#dc0073ff".to_string()
}

fn default_color_5() -> String {
    "#89fc00ff".to_string()
}

impl ThemePalette {
    /// Generates a CSS `:root { ... }` block with all 5 color variables.
    /// Used by the service to inject CSS custom properties via `CssProvider::load_from_data()`.
    pub fn to_css(&self) -> String {
        format!(
            ":root {{\n    --theme-color-1: {};\n    --theme-color-2: {};\n    --theme-color-3: {};\n    --theme-color-4: {};\n    --theme-color-5: {};\n}}",
            self.color_1, self.color_2, self.color_3, self.color_4, self.color_5
        )
    }
}

/// Theme colors for both Dark and Light modes.
///
/// Each mode has its own `ThemePalette` with 5 colors. The service selects
/// the appropriate palette based on the effective mode (Dark or Light) and
/// injects the corresponding CSS custom properties.
///
/// This eliminates the need for separate dark-mode and light-mode themes —
/// a single theme adapts its colors automatically.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ThemeColors {
    /// Color palette for Dark mode.
    /// Defaults to the official Smearor design palette.
    #[serde(default)]
    pub dark: ThemePalette,

    /// Color palette for Light mode.
    /// Defaults to the official Smearor design palette.
    #[serde(default)]
    pub light: ThemePalette,
}

impl ThemeColors {
    /// Returns the palette for the given effective mode.
    /// For Dark mode, returns `self.dark`; for Light mode, returns `self.light`.
    /// For System mode, the caller must resolve to Dark or Light first.
    pub fn palette_for_mode(&self, mode: crate::ThemeMode) -> &ThemePalette {
        match mode {
            crate::ThemeMode::Dark => &self.dark,
            crate::ThemeMode::Light => &self.light,
            crate::ThemeMode::System => &self.dark,
        }
    }

    /// Generates a CSS `:root { ... }` block with all 5 color variables
    /// for the given effective mode.
    /// Used by the service to inject CSS custom properties via `CssProvider::load_from_data()`.
    pub fn to_css(&self, mode: crate::ThemeMode) -> String {
        self.palette_for_mode(mode).to_css()
    }
}
