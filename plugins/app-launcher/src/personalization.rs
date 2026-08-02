use smearor_swipe_launcher_plugin_api::Locale;

/// Personalization override data for the app launcher widget.
///
/// Stores locale received from the personalization service.
/// When available, these values override the default English labels.
#[derive(Clone, Debug, Default)]
pub struct PersonalizationOverride {
    /// Locale for label translations.
    pub locale: Locale,
}
