use smearor_swipe_launcher_plugin_api::Locale;

/// Audio widget labels that can be localized.
#[derive(Copy, Clone, Debug)]
pub enum AudioLabel {
    /// Volume label.
    Volume,
    /// Muted state label.
    Muted,
    /// Mute action label.
    Mute,
    /// Volume up action label.
    VolumeUp,
    /// Volume down action label.
    VolumeDown,
    /// Next device action label.
    NextDevice,
    /// No device available label.
    NoDevice,
}

impl AudioLabel {
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
            AudioLabel::Volume => "Volume",
            AudioLabel::Muted => "Muted",
            AudioLabel::Mute => "Mute",
            AudioLabel::VolumeUp => "Volume Up",
            AudioLabel::VolumeDown => "Volume Down",
            AudioLabel::NextDevice => "Next Device",
            AudioLabel::NoDevice => "No device",
        }
    }

    fn german(&self) -> &'static str {
        match self {
            AudioLabel::Volume => "Lautst\u{e4}rke",
            AudioLabel::Muted => "Stumm",
            AudioLabel::Mute => "Stumm",
            AudioLabel::VolumeUp => "Lauter",
            AudioLabel::VolumeDown => "Leiser",
            AudioLabel::NextDevice => "N\u{e4}chstes Ger\u{e4}t",
            AudioLabel::NoDevice => "Kein Ger\u{e4}t",
        }
    }

    fn french(&self) -> &'static str {
        match self {
            AudioLabel::Volume => "Volume",
            AudioLabel::Muted => "Muet",
            AudioLabel::Mute => "Muet",
            AudioLabel::VolumeUp => "Augmenter",
            AudioLabel::VolumeDown => "Diminuer",
            AudioLabel::NextDevice => "P\u{e9}riph\u{e9}rique suivant",
            AudioLabel::NoDevice => "Aucun p\u{e9}riph\u{e9}rique",
        }
    }

    fn spanish(&self) -> &'static str {
        match self {
            AudioLabel::Volume => "Volumen",
            AudioLabel::Muted => "Silenciado",
            AudioLabel::Mute => "Silenciar",
            AudioLabel::VolumeUp => "Subir volumen",
            AudioLabel::VolumeDown => "Bajar volumen",
            AudioLabel::NextDevice => "Dispositivo siguiente",
            AudioLabel::NoDevice => "Sin dispositivo",
        }
    }

    fn italian(&self) -> &'static str {
        match self {
            AudioLabel::Volume => "Volume",
            AudioLabel::Muted => "Muto",
            AudioLabel::Mute => "Muto",
            AudioLabel::VolumeUp => "Aumenta volume",
            AudioLabel::VolumeDown => "Abbassa volume",
            AudioLabel::NextDevice => "Dispositivo successivo",
            AudioLabel::NoDevice => "Nessun dispositivo",
        }
    }
}
