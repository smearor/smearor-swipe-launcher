use serde::Deserialize;
use serde::Serialize;

/// Available views that the theme switcher widget can display.
/// Each variant corresponds to a data category rendered in the widget tile.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum ThemeView {
    /// Current theme: shows the applied theme name and effective mode.
    /// Clicking cycles to the next theme.
    #[default]
    CurrentTheme,
    /// Theme list: shows the count of configured themes.
    /// Clicking applies the next theme.
    ThemeList,
    /// Mode indicator: shows the effective mode (Dark/Light/System).
    /// Clicking toggles between Dark, Light, and System mode themes.
    ModeIndicator,
}
