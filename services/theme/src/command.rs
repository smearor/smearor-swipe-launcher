/// Internal command enum for the service event loop.
pub enum ThemeCommand {
    /// Select a theme by name (does not apply).
    SelectTheme(String),
    /// Apply the currently selected theme.
    ApplySelected,
    /// Select a theme by name and apply it immediately.
    SelectAndApply(String),
    /// Refresh status and re-broadcast.
    Refresh,
    /// Add a new theme to the configuration.
    AddTheme(smearor_theme_model::Theme),
    /// Remove a theme from the configuration by name.
    RemoveTheme(String),
    /// Personalization color scheme changed — re-evaluate System mode themes.
    ColorSchemeChanged(smearor_personalization_model::ColorScheme),
}
