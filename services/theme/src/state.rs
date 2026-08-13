use smearor_personalization_model::ColorScheme;
use smearor_theme_model::Theme;
use smearor_theme_model::ThemeMode;

/// Runtime state of the theme service.
/// Note: CSS providers are NOT stored here because `CssProvider` is not `Send`.
/// Provider management happens entirely on the GTK main thread via `idle_add_once`.
pub struct ThemeState {
    /// All configured themes loaded from themes.toml.
    pub themes: Vec<Theme>,
    /// Index of the currently selected theme.
    pub selected_theme_index: usize,
    /// Name of the currently applied theme.
    pub current_theme: Option<String>,
    /// Number of CSS providers currently applied (for tracking/removal on next switch).
    pub applied_provider_count: usize,
    /// Current effective mode (Dark or Light after System resolution).
    pub effective_mode: ThemeMode,
    /// Latest personalization color scheme (for System mode resolution).
    pub system_color_scheme: ColorScheme,
}

impl Default for ThemeState {
    fn default() -> Self {
        Self {
            themes: Vec::new(),
            selected_theme_index: 0,
            current_theme: None,
            applied_provider_count: 0,
            effective_mode: ThemeMode::Dark,
            system_color_scheme: ColorScheme::Dark,
        }
    }
}
