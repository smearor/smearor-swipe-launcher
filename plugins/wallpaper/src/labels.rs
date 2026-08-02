use smearor_swipe_launcher_plugin_api::Locale;

/// Wallpaper widget labels that can be localized.
#[derive(Copy, Clone, Debug)]
pub enum WallpaperLabel {
    /// Label for the light color scheme.
    Light,
    /// Label for the dark color scheme.
    Dark,
    /// Label for the system color scheme.
    System,
    /// Label for the next wallpaper action.
    Next,
    /// Label for the previous wallpaper action.
    Previous,
    /// Label for the random wallpaper action.
    Random,
    /// Label for the current wallpaper.
    Current,
    /// Label for the wallpaper selector.
    Selector,
    /// Label shown when no theme is available.
    NoTheme,
    /// Label for the running status.
    Running,
    /// Label for the stopped status.
    Stopped,
}

impl WallpaperLabel {
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
            WallpaperLabel::Light => "Light".to_string(),
            WallpaperLabel::Dark => "Dark".to_string(),
            WallpaperLabel::System => "System".to_string(),
            WallpaperLabel::Next => "Next".to_string(),
            WallpaperLabel::Previous => "Previous".to_string(),
            WallpaperLabel::Random => "Random".to_string(),
            WallpaperLabel::Current => "Current".to_string(),
            WallpaperLabel::Selector => "Selector".to_string(),
            WallpaperLabel::NoTheme => "No theme".to_string(),
            WallpaperLabel::Running => "Running".to_string(),
            WallpaperLabel::Stopped => "Stopped".to_string(),
        }
    }

    fn german(&self) -> String {
        match self {
            WallpaperLabel::Light => "Hell".to_string(),
            WallpaperLabel::Dark => "Dunkel".to_string(),
            WallpaperLabel::System => "System".to_string(),
            WallpaperLabel::Next => "Weiter".to_string(),
            WallpaperLabel::Previous => "Zur\u{fc}ck".to_string(),
            WallpaperLabel::Random => "Zuf\u{e4}llig".to_string(),
            WallpaperLabel::Current => "Aktuell".to_string(),
            WallpaperLabel::Selector => "Auswahl".to_string(),
            WallpaperLabel::NoTheme => "Kein Theme".to_string(),
            WallpaperLabel::Running => "Aktiv".to_string(),
            WallpaperLabel::Stopped => "Gestoppt".to_string(),
        }
    }

    fn french(&self) -> String {
        match self {
            WallpaperLabel::Light => "Clair".to_string(),
            WallpaperLabel::Dark => "Sombre".to_string(),
            WallpaperLabel::System => "Syst\u{e8}me".to_string(),
            WallpaperLabel::Next => "Suivant".to_string(),
            WallpaperLabel::Previous => "Pr\u{e9}c\u{e9}dent".to_string(),
            WallpaperLabel::Random => "Al\u{e9}atoire".to_string(),
            WallpaperLabel::Current => "Actuel".to_string(),
            WallpaperLabel::Selector => "S\u{e9}lecteur".to_string(),
            WallpaperLabel::NoTheme => "Aucun th\u{e8}me".to_string(),
            WallpaperLabel::Running => "En cours".to_string(),
            WallpaperLabel::Stopped => "Arr\u{ea}t\u{e9}".to_string(),
        }
    }

    fn spanish(&self) -> String {
        match self {
            WallpaperLabel::Light => "Claro".to_string(),
            WallpaperLabel::Dark => "Oscuro".to_string(),
            WallpaperLabel::System => "Sistema".to_string(),
            WallpaperLabel::Next => "Siguiente".to_string(),
            WallpaperLabel::Previous => "Anterior".to_string(),
            WallpaperLabel::Random => "Aleatorio".to_string(),
            WallpaperLabel::Current => "Actual".to_string(),
            WallpaperLabel::Selector => "Selector".to_string(),
            WallpaperLabel::NoTheme => "Sin tema".to_string(),
            WallpaperLabel::Running => "En ejecuci\u{f3}n".to_string(),
            WallpaperLabel::Stopped => "Detenido".to_string(),
        }
    }

    fn italian(&self) -> String {
        match self {
            WallpaperLabel::Light => "Chiaro".to_string(),
            WallpaperLabel::Dark => "Scuro".to_string(),
            WallpaperLabel::System => "Sistema".to_string(),
            WallpaperLabel::Next => "Avanti".to_string(),
            WallpaperLabel::Previous => "Indietro".to_string(),
            WallpaperLabel::Random => "Casuale".to_string(),
            WallpaperLabel::Current => "Attuale".to_string(),
            WallpaperLabel::Selector => "Selettore".to_string(),
            WallpaperLabel::NoTheme => "Nessun tema".to_string(),
            WallpaperLabel::Running => "In esecuzione".to_string(),
            WallpaperLabel::Stopped => "Fermato".to_string(),
        }
    }
}
