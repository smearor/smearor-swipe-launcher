use smearor_swipe_launcher_plugin_api::Locale;

/// Voice assistant widget labels that can be localized.
#[derive(Copy, Clone, Debug)]
pub enum VoiceAssistantLabel {
    /// "Idle" status label.
    Idle,
    /// "Standby" status label.
    Standby,
    /// "Listening..." status label.
    Listening,
    /// "Transcribing..." status label.
    Transcribing,
    /// "Thinking..." status label.
    Thinking,
    /// "Executing..." status label.
    Executing,
    /// "Speaking..." status label.
    Speaking,
    /// "Error" status label.
    Error,
    /// "Listen" atomic widget label.
    Listen,
    /// "PTT" (Push-to-Talk) atomic widget label.
    Ptt,
    /// "Stop" atomic widget label.
    Stop,
    /// "Status" atomic widget label.
    Status,
}

impl VoiceAssistantLabel {
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
            Self::Idle => "Idle",
            Self::Standby => "Standby",
            Self::Listening => "Listening...",
            Self::Transcribing => "Transcribing...",
            Self::Thinking => "Thinking...",
            Self::Executing => "Executing...",
            Self::Speaking => "Speaking...",
            Self::Error => "Error",
            Self::Listen => "Listen",
            Self::Ptt => "PTT",
            Self::Stop => "Stop",
            Self::Status => "Status",
        }
    }

    fn german(&self) -> &'static str {
        match self {
            Self::Idle => "Leerlauf",
            Self::Standby => "Bereit",
            Self::Listening => "H\u{f6}re zu...",
            Self::Transcribing => "Transkribiere...",
            Self::Thinking => "Denke nach...",
            Self::Executing => "F\u{fc}hre aus...",
            Self::Speaking => "Spreche...",
            Self::Error => "Fehler",
            Self::Listen => "H\u{f6}ren",
            Self::Ptt => "PTT",
            Self::Stop => "Stopp",
            Self::Status => "Status",
        }
    }

    fn french(&self) -> &'static str {
        match self {
            Self::Idle => "Inactif",
            Self::Standby => "En attente",
            Self::Listening => "\u{c9}coute...",
            Self::Transcribing => "Transcription...",
            Self::Thinking => "R\u{e9}flexion...",
            Self::Executing => "Ex\u{e9}cution...",
            Self::Speaking => "Parole...",
            Self::Error => "Erreur",
            Self::Listen => "\u{c9}couter",
            Self::Ptt => "PTT",
            Self::Stop => "Arr\u{ea}ter",
            Self::Status => "Statut",
        }
    }

    fn spanish(&self) -> &'static str {
        match self {
            Self::Idle => "Inactivo",
            Self::Standby => "En espera",
            Self::Listening => "Escuchando...",
            Self::Transcribing => "Transcribiendo...",
            Self::Thinking => "Pensando...",
            Self::Executing => "Ejecutando...",
            Self::Speaking => "Hablando...",
            Self::Error => "Error",
            Self::Listen => "Escuchar",
            Self::Ptt => "PTT",
            Self::Stop => "Detener",
            Self::Status => "Estado",
        }
    }

    fn italian(&self) -> &'static str {
        match self {
            Self::Idle => "Inattivo",
            Self::Standby => "In attesa",
            Self::Listening => "In ascolto...",
            Self::Transcribing => "Trascrizione...",
            Self::Thinking => "Pensando...",
            Self::Executing => "Esecuzione...",
            Self::Speaking => "Parlando...",
            Self::Error => "Errore",
            Self::Listen => "Ascolta",
            Self::Ptt => "PTT",
            Self::Stop => "Ferma",
            Self::Status => "Stato",
        }
    }
}
