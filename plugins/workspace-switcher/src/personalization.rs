use smearor_swipe_launcher_plugin_api::Locale;

/// Personalization override data for the workspace switcher widget.
///
/// Stores the locale received from the personalization service.
/// When available, workspace names are sorted according to locale
/// collation rules.
#[derive(Clone, Debug, Default)]
pub struct PersonalizationOverride {
    /// Locale for workspace name sorting.
    pub locale: Locale,
}
