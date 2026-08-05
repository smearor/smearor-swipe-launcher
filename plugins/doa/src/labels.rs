use smearor_doa_model::DoaDirection;
use smearor_swipe_launcher_plugin_api::Locale;

/// DoA widget labels that can be localized.
#[derive(Copy, Clone, Debug)]
pub enum DoaLabel {
    /// Direction: North.
    North,
    /// Direction: East.
    East,
    /// Direction: South.
    South,
    /// Direction: West.
    West,
    /// Device disconnected label.
    Disconnected,
    /// Speech detected label.
    SpeechDetected,
    /// No speech label.
    Silence,
    /// Device info label.
    DeviceInfo,
    /// Compass view label.
    Compass,
    /// Direction view label.
    Direction,
}

impl From<DoaDirection> for DoaLabel {
    fn from(direction: DoaDirection) -> Self {
        match direction {
            DoaDirection::North => Self::North,
            DoaDirection::East => Self::East,
            DoaDirection::South => Self::South,
            DoaDirection::West => Self::West,
        }
    }
}

impl DoaLabel {
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
            DoaLabel::North => "North",
            DoaLabel::East => "East",
            DoaLabel::South => "South",
            DoaLabel::West => "West",
            DoaLabel::Disconnected => "Disconnected",
            DoaLabel::SpeechDetected => "Speech",
            DoaLabel::Silence => "Silence",
            DoaLabel::DeviceInfo => "Device",
            DoaLabel::Compass => "Compass",
            DoaLabel::Direction => "Direction",
        }
    }

    fn german(&self) -> &'static str {
        match self {
            DoaLabel::North => "Norden",
            DoaLabel::East => "Osten",
            DoaLabel::South => "S\u{fc}den",
            DoaLabel::West => "Westen",
            DoaLabel::Disconnected => "Getrennt",
            DoaLabel::SpeechDetected => "Sprache",
            DoaLabel::Silence => "Stille",
            DoaLabel::DeviceInfo => "Ger\u{e4}t",
            DoaLabel::Compass => "Kompass",
            DoaLabel::Direction => "Richtung",
        }
    }

    fn french(&self) -> &'static str {
        match self {
            DoaLabel::North => "Nord",
            DoaLabel::East => "Est",
            DoaLabel::South => "Sud",
            DoaLabel::West => "Ouest",
            DoaLabel::Disconnected => "D\u{e9}connect\u{e9}",
            DoaLabel::SpeechDetected => "Parole",
            DoaLabel::Silence => "Silence",
            DoaLabel::DeviceInfo => "Appareil",
            DoaLabel::Compass => "Boussole",
            DoaLabel::Direction => "Direction",
        }
    }

    fn spanish(&self) -> &'static str {
        match self {
            DoaLabel::North => "Norte",
            DoaLabel::East => "Este",
            DoaLabel::South => "Sur",
            DoaLabel::West => "Oeste",
            DoaLabel::Disconnected => "Desconectado",
            DoaLabel::SpeechDetected => "Voz",
            DoaLabel::Silence => "Silencio",
            DoaLabel::DeviceInfo => "Dispositivo",
            DoaLabel::Compass => "Br\u{fa}jula",
            DoaLabel::Direction => "Direcci\u{f3}n",
        }
    }

    fn italian(&self) -> &'static str {
        match self {
            DoaLabel::North => "Nord",
            DoaLabel::East => "Est",
            DoaLabel::South => "Sud",
            DoaLabel::West => "Ovest",
            DoaLabel::Disconnected => "Disconnesso",
            DoaLabel::SpeechDetected => "Voce",
            DoaLabel::Silence => "Silenzio",
            DoaLabel::DeviceInfo => "Dispositivo",
            DoaLabel::Compass => "Bussola",
            DoaLabel::Direction => "Direzione",
        }
    }
}
