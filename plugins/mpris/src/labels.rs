use smearor_swipe_launcher_plugin_api::Locale;

/// MPRIS widget labels that can be localized.
#[derive(Copy, Clone, Debug)]
pub enum MprisLabel {
    /// Unknown artist label.
    UnknownArtist,
    /// Unknown title label.
    UnknownTitle,
    /// Unknown album label.
    UnknownAlbum,
    /// No player available label.
    NoPlayer,
    /// Playing status label.
    Playing,
    /// Paused status label.
    Paused,
    /// Stopped status label.
    Stopped,
}

impl MprisLabel {
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
            MprisLabel::UnknownArtist => "Unknown artist",
            MprisLabel::UnknownTitle => "Unknown title",
            MprisLabel::UnknownAlbum => "Unknown album",
            MprisLabel::NoPlayer => "No player",
            MprisLabel::Playing => "Playing",
            MprisLabel::Paused => "Paused",
            MprisLabel::Stopped => "Stopped",
        }
    }

    fn german(&self) -> &'static str {
        match self {
            MprisLabel::UnknownArtist => "Unbekannter K\u{fc}nstler",
            MprisLabel::UnknownTitle => "Unbekannter Titel",
            MprisLabel::UnknownAlbum => "Unbekanntes Album",
            MprisLabel::NoPlayer => "Kein Player",
            MprisLabel::Playing => "Wiedergabe",
            MprisLabel::Paused => "Pausiert",
            MprisLabel::Stopped => "Gestoppt",
        }
    }

    fn french(&self) -> &'static str {
        match self {
            MprisLabel::UnknownArtist => "Artiste inconnu",
            MprisLabel::UnknownTitle => "Titre inconnu",
            MprisLabel::UnknownAlbum => "Album inconnu",
            MprisLabel::NoPlayer => "Aucun lecteur",
            MprisLabel::Playing => "Lecture",
            MprisLabel::Paused => "En pause",
            MprisLabel::Stopped => "Arr\u{ea}t\u{e9}",
        }
    }

    fn spanish(&self) -> &'static str {
        match self {
            MprisLabel::UnknownArtist => "Artista desconocido",
            MprisLabel::UnknownTitle => "T\u{ed}tulo desconocido",
            MprisLabel::UnknownAlbum => "\u{c1}lbum desconocido",
            MprisLabel::NoPlayer => "Sin reproductor",
            MprisLabel::Playing => "Reproduciendo",
            MprisLabel::Paused => "En pausa",
            MprisLabel::Stopped => "Detenido",
        }
    }

    fn italian(&self) -> &'static str {
        match self {
            MprisLabel::UnknownArtist => "Artista sconosciuto",
            MprisLabel::UnknownTitle => "Titolo sconosciuto",
            MprisLabel::UnknownAlbum => "Album sconosciuto",
            MprisLabel::NoPlayer => "Nessun lettore",
            MprisLabel::Playing => "In riproduzione",
            MprisLabel::Paused => "In pausa",
            MprisLabel::Stopped => "Fermato",
        }
    }
}
