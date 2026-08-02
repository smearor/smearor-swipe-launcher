use smearor_swipe_launcher_plugin_api::Locale;

/// App launcher labels that can be localized.
///
/// Covers category names and search-related labels used by the app launcher.
#[derive(Copy, Clone, Debug)]
pub enum AppLauncherLabel {
    /// The "Development" category.
    Development,
    /// The "Games" category.
    Games,
    /// The "Graphics" category.
    Graphics,
    /// The "Internet" category.
    Internet,
    /// The "Multimedia" category.
    Multimedia,
    /// The "Office" category.
    Office,
    /// The "Settings" category.
    Settings,
    /// The "System" category.
    System,
    /// The "Utilities" category.
    Utilities,
    /// The "Search" label.
    Search,
    /// The "No results found" message.
    NoResults,
}

impl AppLauncherLabel {
    /// Returns a localized label for the given key and locale.
    /// Falls back to English when the locale is not supported.
    pub fn localized_label(&self, locale: Locale) -> &'static str {
        match locale {
            Locale::DeDe => self.german(),
            Locale::FrFr => self.french(),
            Locale::ItIt => self.italian(),
            Locale::EsEs => self.spanish(),
            _ => self.english(),
        }
    }

    fn english(&self) -> &'static str {
        match self {
            AppLauncherLabel::Development => "Development",
            AppLauncherLabel::Games => "Games",
            AppLauncherLabel::Graphics => "Graphics",
            AppLauncherLabel::Internet => "Internet",
            AppLauncherLabel::Multimedia => "Multimedia",
            AppLauncherLabel::Office => "Office",
            AppLauncherLabel::Settings => "Settings",
            AppLauncherLabel::System => "System",
            AppLauncherLabel::Utilities => "Utilities",
            AppLauncherLabel::Search => "Search",
            AppLauncherLabel::NoResults => "No results found",
        }
    }

    fn german(&self) -> &'static str {
        match self {
            AppLauncherLabel::Development => "Entwicklung",
            AppLauncherLabel::Games => "Spiele",
            AppLauncherLabel::Graphics => "Grafik",
            AppLauncherLabel::Internet => "Internet",
            AppLauncherLabel::Multimedia => "Multimedia",
            AppLauncherLabel::Office => "B\u{fc}ro",
            AppLauncherLabel::Settings => "Einstellungen",
            AppLauncherLabel::System => "System",
            AppLauncherLabel::Utilities => "Dienstprogramme",
            AppLauncherLabel::Search => "Suchen",
            AppLauncherLabel::NoResults => "Keine Ergebnisse",
        }
    }

    fn french(&self) -> &'static str {
        match self {
            AppLauncherLabel::Development => "D\u{e9}veloppement",
            AppLauncherLabel::Games => "Jeux",
            AppLauncherLabel::Graphics => "Graphisme",
            AppLauncherLabel::Internet => "Internet",
            AppLauncherLabel::Multimedia => "Multim\u{e9}dia",
            AppLauncherLabel::Office => "Bureau",
            AppLauncherLabel::Settings => "Param\u{e8}tres",
            AppLauncherLabel::System => "Syst\u{e8}me",
            AppLauncherLabel::Utilities => "Utilitaires",
            AppLauncherLabel::Search => "Rechercher",
            AppLauncherLabel::NoResults => "Aucun r\u{e9}sultat",
        }
    }

    fn spanish(&self) -> &'static str {
        match self {
            AppLauncherLabel::Development => "Desarrollo",
            AppLauncherLabel::Games => "Juegos",
            AppLauncherLabel::Graphics => "Gr\u{e1}ficos",
            AppLauncherLabel::Internet => "Internet",
            AppLauncherLabel::Multimedia => "Multimedia",
            AppLauncherLabel::Office => "Oficina",
            AppLauncherLabel::Settings => "Configuraci\u{f3}n",
            AppLauncherLabel::System => "Sistema",
            AppLauncherLabel::Utilities => "Utilidades",
            AppLauncherLabel::Search => "Buscar",
            AppLauncherLabel::NoResults => "Sin resultados",
        }
    }

    fn italian(&self) -> &'static str {
        match self {
            AppLauncherLabel::Development => "Sviluppo",
            AppLauncherLabel::Games => "Giochi",
            AppLauncherLabel::Graphics => "Grafica",
            AppLauncherLabel::Internet => "Internet",
            AppLauncherLabel::Multimedia => "Multimedia",
            AppLauncherLabel::Office => "Ufficio",
            AppLauncherLabel::Settings => "Impostazioni",
            AppLauncherLabel::System => "Sistema",
            AppLauncherLabel::Utilities => "Utilit\u{e0}",
            AppLauncherLabel::Search => "Cerca",
            AppLauncherLabel::NoResults => "Nessun risultato",
        }
    }
}
