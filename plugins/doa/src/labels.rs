use smearor_doa_model::DoaDirection;
use smearor_swipe_launcher_plugin_api::Locale;

/// DoA widget labels that can be localized.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
    /// Paused state label.
    Paused,
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
            DoaLabel::Paused => "Paused",
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
            DoaLabel::Paused => "Pausiert",
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
            DoaLabel::Paused => "En pause",
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
            DoaLabel::Paused => "Pausado",
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
            DoaLabel::Paused => "In pausa",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smearor_doa_model::DoaDirection;

    #[test]
    fn test_english_labels() {
        let locale = Locale::EnUs;
        assert_eq!(DoaLabel::North.localized_label(locale), "North");
        assert_eq!(DoaLabel::East.localized_label(locale), "East");
        assert_eq!(DoaLabel::South.localized_label(locale), "South");
        assert_eq!(DoaLabel::West.localized_label(locale), "West");
        assert_eq!(DoaLabel::Disconnected.localized_label(locale), "Disconnected");
        assert_eq!(DoaLabel::SpeechDetected.localized_label(locale), "Speech");
        assert_eq!(DoaLabel::Silence.localized_label(locale), "Silence");
        assert_eq!(DoaLabel::DeviceInfo.localized_label(locale), "Device");
        assert_eq!(DoaLabel::Compass.localized_label(locale), "Compass");
        assert_eq!(DoaLabel::Direction.localized_label(locale), "Direction");
        assert_eq!(DoaLabel::Paused.localized_label(locale), "Paused");
    }

    #[test]
    fn test_german_labels() {
        let locale = Locale::DeDe;
        assert_eq!(DoaLabel::North.localized_label(locale), "Norden");
        assert_eq!(DoaLabel::East.localized_label(locale), "Osten");
        assert_eq!(DoaLabel::South.localized_label(locale), "S\u{fc}den");
        assert_eq!(DoaLabel::West.localized_label(locale), "Westen");
        assert_eq!(DoaLabel::Disconnected.localized_label(locale), "Getrennt");
        assert_eq!(DoaLabel::SpeechDetected.localized_label(locale), "Sprache");
        assert_eq!(DoaLabel::Silence.localized_label(locale), "Stille");
        assert_eq!(DoaLabel::DeviceInfo.localized_label(locale), "Ger\u{e4}t");
        assert_eq!(DoaLabel::Compass.localized_label(locale), "Kompass");
        assert_eq!(DoaLabel::Direction.localized_label(locale), "Richtung");
        assert_eq!(DoaLabel::Paused.localized_label(locale), "Pausiert");
    }

    #[test]
    fn test_french_labels() {
        let locale = Locale::FrFr;
        assert_eq!(DoaLabel::North.localized_label(locale), "Nord");
        assert_eq!(DoaLabel::East.localized_label(locale), "Est");
        assert_eq!(DoaLabel::South.localized_label(locale), "Sud");
        assert_eq!(DoaLabel::West.localized_label(locale), "Ouest");
        assert_eq!(DoaLabel::Disconnected.localized_label(locale), "D\u{e9}connect\u{e9}");
        assert_eq!(DoaLabel::SpeechDetected.localized_label(locale), "Parole");
        assert_eq!(DoaLabel::Silence.localized_label(locale), "Silence");
        assert_eq!(DoaLabel::DeviceInfo.localized_label(locale), "Appareil");
        assert_eq!(DoaLabel::Compass.localized_label(locale), "Boussole");
        assert_eq!(DoaLabel::Direction.localized_label(locale), "Direction");
        assert_eq!(DoaLabel::Paused.localized_label(locale), "En pause");
    }

    #[test]
    fn test_spanish_labels() {
        let locale = Locale::EsEs;
        assert_eq!(DoaLabel::North.localized_label(locale), "Norte");
        assert_eq!(DoaLabel::East.localized_label(locale), "Este");
        assert_eq!(DoaLabel::South.localized_label(locale), "Sur");
        assert_eq!(DoaLabel::West.localized_label(locale), "Oeste");
        assert_eq!(DoaLabel::Disconnected.localized_label(locale), "Desconectado");
        assert_eq!(DoaLabel::SpeechDetected.localized_label(locale), "Voz");
        assert_eq!(DoaLabel::Silence.localized_label(locale), "Silencio");
        assert_eq!(DoaLabel::DeviceInfo.localized_label(locale), "Dispositivo");
        assert_eq!(DoaLabel::Compass.localized_label(locale), "Br\u{fa}jula");
        assert_eq!(DoaLabel::Direction.localized_label(locale), "Direcci\u{f3}n");
        assert_eq!(DoaLabel::Paused.localized_label(locale), "Pausado");
    }

    #[test]
    fn test_italian_labels() {
        let locale = Locale::ItIt;
        assert_eq!(DoaLabel::North.localized_label(locale), "Nord");
        assert_eq!(DoaLabel::East.localized_label(locale), "Est");
        assert_eq!(DoaLabel::South.localized_label(locale), "Sud");
        assert_eq!(DoaLabel::West.localized_label(locale), "Ovest");
        assert_eq!(DoaLabel::Disconnected.localized_label(locale), "Disconnesso");
        assert_eq!(DoaLabel::SpeechDetected.localized_label(locale), "Voce");
        assert_eq!(DoaLabel::Silence.localized_label(locale), "Silenzio");
        assert_eq!(DoaLabel::DeviceInfo.localized_label(locale), "Dispositivo");
        assert_eq!(DoaLabel::Compass.localized_label(locale), "Bussola");
        assert_eq!(DoaLabel::Direction.localized_label(locale), "Direzione");
        assert_eq!(DoaLabel::Paused.localized_label(locale), "In pausa");
    }

    #[test]
    fn test_unsupported_locale_falls_back_to_english() {
        let locale = Locale::Unknown;
        assert_eq!(DoaLabel::North.localized_label(locale), "North");
        assert_eq!(DoaLabel::Disconnected.localized_label(locale), "Disconnected");
        assert_eq!(DoaLabel::Paused.localized_label(locale), "Paused");
    }

    #[test]
    fn test_from_doa_direction() {
        assert_eq!(DoaLabel::from(DoaDirection::North), DoaLabel::North);
        assert_eq!(DoaLabel::from(DoaDirection::East), DoaLabel::East);
        assert_eq!(DoaLabel::from(DoaDirection::South), DoaLabel::South);
        assert_eq!(DoaLabel::from(DoaDirection::West), DoaLabel::West);
    }

    #[test]
    fn test_all_labels_non_empty_for_all_locales() {
        let labels = [
            DoaLabel::North,
            DoaLabel::East,
            DoaLabel::South,
            DoaLabel::West,
            DoaLabel::Disconnected,
            DoaLabel::SpeechDetected,
            DoaLabel::Silence,
            DoaLabel::DeviceInfo,
            DoaLabel::Compass,
            DoaLabel::Direction,
            DoaLabel::Paused,
        ];
        let locales = [Locale::EnUs, Locale::DeDe, Locale::FrFr, Locale::EsEs, Locale::ItIt];
        for &label in &labels {
            for &locale in &locales {
                assert!(!label.localized_label(locale).is_empty(), "Label {:?} for locale {:?} is empty", label, locale);
            }
        }
    }
}
