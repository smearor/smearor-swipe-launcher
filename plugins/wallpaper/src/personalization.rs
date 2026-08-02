use smearor_personalization_model::ColorScheme;
use smearor_swipe_launcher_plugin_api::Locale;

/// Personalization override data for the wallpaper widget.
///
/// Stores color scheme and locale received from the personalization service.
/// When available, these values override the default system color scheme and
/// English labels.
#[derive(Clone, Debug, Default)]
pub struct PersonalizationOverride {
    /// Preferred color scheme (light, dark, or system).
    pub color_scheme: Option<ColorScheme>,
    /// Locale for label translations.
    pub locale: Locale,
}

impl PersonalizationOverride {
    /// Returns the effective color scheme, falling back to system.
    pub fn effective_color_scheme(&self) -> ColorScheme {
        self.color_scheme.clone().unwrap_or_default()
    }
}
