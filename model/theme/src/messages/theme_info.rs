use serde::Deserialize;
use serde::Serialize;

use crate::ThemeColors;
use crate::ThemeMode;
use crate::ThemePalette;

/// FFI-safe theme palette with 5 stabby string colors.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ThemePaletteStabby {
    /// Primary color. Exported as `--theme-color-1`.
    pub color_1: stabby::string::String,
    /// Secondary color. Exported as `--theme-color-2`.
    pub color_2: stabby::string::String,
    /// Tertiary color. Exported as `--theme-color-3`.
    pub color_3: stabby::string::String,
    /// Quaternary color. Exported as `--theme-color-4`.
    pub color_4: stabby::string::String,
    /// Quinary color. Exported as `--theme-color-5`.
    pub color_5: stabby::string::String,
}

impl From<&ThemePalette> for ThemePaletteStabby {
    fn from(p: &ThemePalette) -> Self {
        Self {
            color_1: p.color_1.as_str().into(),
            color_2: p.color_2.as_str().into(),
            color_3: p.color_3.as_str().into(),
            color_4: p.color_4.as_str().into(),
            color_5: p.color_5.as_str().into(),
        }
    }
}

/// FFI-safe theme colors for Dark and Light modes.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ThemeColorsStabby {
    /// Color palette for Dark mode.
    pub dark: ThemePaletteStabby,
    /// Color palette for Light mode.
    pub light: ThemePaletteStabby,
}

impl From<&ThemeColors> for ThemeColorsStabby {
    fn from(c: &ThemeColors) -> Self {
        Self {
            dark: ThemePaletteStabby::from(&c.dark),
            light: ThemePaletteStabby::from(&c.light),
        }
    }
}

/// Lightweight theme info included in status messages.
/// Contains only display-relevant fields, not full CSS file paths.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ThemeInfo {
    /// Theme name.
    pub name: stabby::string::String,
    /// Theme description.
    pub description: stabby::string::String,
    /// Nerd Font preview icon name.
    pub preview_icon: stabby::string::String,
    /// Optional path to a preview image shown in the widget tile.
    pub preview_image_path: stabby::string::String,
    /// Theme colors for Dark and Light modes (5 hex strings each).
    pub colors: ThemeColorsStabby,
    /// Theme mode (Dark, Light, System).
    pub mode: ThemeMode,
    /// Whether this theme is coupled with a wallpaper theme.
    pub has_wallpaper: bool,
}
