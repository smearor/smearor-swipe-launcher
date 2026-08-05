use smearor_swipe_launcher_plugin_api::Locale;

/// Personalization override data for the DoA widget.
///
/// Stores locale received from the personalization service.
/// When available, locale determines label translations for the widget.
#[derive(Clone, Debug, Default)]
pub struct PersonalizationOverride {
    /// Locale for label translations.
    pub locale: Locale,
}

impl PersonalizationOverride {
    /// Returns the effective locale, falling back to default (English).
    pub fn effective_locale(&self) -> Locale {
        self.locale
    }
}
