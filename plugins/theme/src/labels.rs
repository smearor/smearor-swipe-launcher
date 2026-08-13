use smearor_swipe_launcher_plugin_api::Locale;
use std::str::FromStr;

/// Theme widget labels that can be localized.
#[derive(Copy, Clone, Debug)]
pub enum ThemeLabel {
    /// Label for the theme concept.
    Theme,
    /// Label for the themes plural.
    Themes,
    /// Label shown when no theme is available.
    NoTheme,
    /// Label for the mode concept.
    Mode,
    /// Label for the dark color scheme.
    Dark,
    /// Label for the light color scheme.
    Light,
    /// Label for the system color scheme.
    System,
    /// Label for the applied status.
    Applied,
}

impl ThemeLabel {
    /// Returns a localized label for the given key and locale.
    /// Falls back to English when the locale is not supported.
    pub fn localized_label(&self, locale: Locale) -> String {
        match locale {
            Locale::DeDe => self.german(),
            Locale::FrFr => self.french(),
            Locale::ItIt => self.italian(),
            Locale::EsEs => self.spanish(),
            _ => self.english(),
        }
    }

    fn english(&self) -> String {
        match self {
            ThemeLabel::Theme => "Theme".to_string(),
            ThemeLabel::Themes => "themes".to_string(),
            ThemeLabel::NoTheme => "No theme".to_string(),
            ThemeLabel::Mode => "Mode".to_string(),
            ThemeLabel::Dark => "Dark".to_string(),
            ThemeLabel::Light => "Light".to_string(),
            ThemeLabel::System => "System".to_string(),
            ThemeLabel::Applied => "Applied".to_string(),
        }
    }

    fn german(&self) -> String {
        match self {
            ThemeLabel::Theme => "Theme".to_string(),
            ThemeLabel::Themes => "Themes".to_string(),
            ThemeLabel::NoTheme => "Kein Theme".to_string(),
            ThemeLabel::Mode => "Modus".to_string(),
            ThemeLabel::Dark => "Dunkel".to_string(),
            ThemeLabel::Light => "Hell".to_string(),
            ThemeLabel::System => "System".to_string(),
            ThemeLabel::Applied => "Angewendet".to_string(),
        }
    }

    fn french(&self) -> String {
        match self {
            ThemeLabel::Theme => "Th\u{e8}me".to_string(),
            ThemeLabel::Themes => "th\u{e8}mes".to_string(),
            ThemeLabel::NoTheme => "Pas de th\u{e8}me".to_string(),
            ThemeLabel::Mode => "Mode".to_string(),
            ThemeLabel::Dark => "Sombre".to_string(),
            ThemeLabel::Light => "Clair".to_string(),
            ThemeLabel::System => "Syst\u{e8}me".to_string(),
            ThemeLabel::Applied => "Appliqu\u{e9}".to_string(),
        }
    }

    fn italian(&self) -> String {
        match self {
            ThemeLabel::Theme => "Tema".to_string(),
            ThemeLabel::Themes => "temi".to_string(),
            ThemeLabel::NoTheme => "Nessun tema".to_string(),
            ThemeLabel::Mode => "Modalit\u{e0}".to_string(),
            ThemeLabel::Dark => "Scuro".to_string(),
            ThemeLabel::Light => "Chiaro".to_string(),
            ThemeLabel::System => "Sistema".to_string(),
            ThemeLabel::Applied => "Applicato".to_string(),
        }
    }

    fn spanish(&self) -> String {
        match self {
            ThemeLabel::Theme => "Tema".to_string(),
            ThemeLabel::Themes => "temas".to_string(),
            ThemeLabel::NoTheme => "Sin tema".to_string(),
            ThemeLabel::Mode => "Modo".to_string(),
            ThemeLabel::Dark => "Oscuro".to_string(),
            ThemeLabel::Light => "Claro".to_string(),
            ThemeLabel::System => "Sistema".to_string(),
            ThemeLabel::Applied => "Aplicado".to_string(),
        }
    }
}

/// All localized labels needed by the theme widget.
#[allow(dead_code)]
pub struct ThemeLabels {
    pub theme: String,
    pub themes: String,
    pub no_theme: String,
    pub mode: String,
    pub dark: String,
    pub light: String,
    pub system: String,
    pub applied: String,
}

impl ThemeLabels {
    pub fn from_personalization(p: Option<&smearor_personalization_model::PersonalizationStatusMessage>) -> Self {
        let locale = p
            .and_then(|s| s.locale.as_ref().map(|l| l.to_string()))
            .map(|l| Locale::from_str(&l).unwrap_or_default())
            .unwrap_or_default();
        Self {
            theme: ThemeLabel::Theme.localized_label(locale),
            themes: ThemeLabel::Themes.localized_label(locale),
            no_theme: ThemeLabel::NoTheme.localized_label(locale),
            mode: ThemeLabel::Mode.localized_label(locale),
            dark: ThemeLabel::Dark.localized_label(locale),
            light: ThemeLabel::Light.localized_label(locale),
            system: ThemeLabel::System.localized_label(locale),
            applied: ThemeLabel::Applied.localized_label(locale),
        }
    }
}
