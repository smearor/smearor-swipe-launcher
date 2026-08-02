use smearor_swipe_launcher_plugin_api::Locale;

/// A weekday with localized name support.
///
/// Wraps `time::Weekday` and provides methods to retrieve the weekday name
/// in different languages (German, English, French, Spanish, Italian).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalizedWeekday {
    /// Monday
    Monday,
    /// Tuesday
    Tuesday,
    /// Wednesday
    Wednesday,
    /// Thursday
    Thursday,
    /// Friday
    Friday,
    /// Saturday
    Saturday,
    /// Sunday
    Sunday,
}

impl LocalizedWeekday {
    /// Creates a `LocalizedWeekday` from a `time::Weekday`.
    pub fn from_time_weekday(weekday: time::Weekday) -> Self {
        match weekday {
            time::Weekday::Monday => Self::Monday,
            time::Weekday::Tuesday => Self::Tuesday,
            time::Weekday::Wednesday => Self::Wednesday,
            time::Weekday::Thursday => Self::Thursday,
            time::Weekday::Friday => Self::Friday,
            time::Weekday::Saturday => Self::Saturday,
            time::Weekday::Sunday => Self::Sunday,
        }
    }

    /// Returns the weekday name in the language determined by the given locale.
    pub fn localized(&self, locale: Locale) -> &'static str {
        match locale {
            Locale::EnUs => self.english(),
            Locale::DeDe => self.german(),
            Locale::FrFr => self.french(),
            Locale::ItIt => self.italian(),
            Locale::EsEs => self.spanish(),
            Locale::Unknown => self.german(),
            _ => self.german(),
        }
    }

    /// Returns the weekday name in German (e.g. "Montag", "Dienstag").
    pub fn german(&self) -> &'static str {
        match self {
            Self::Monday => "Montag",
            Self::Tuesday => "Dienstag",
            Self::Wednesday => "Mittwoch",
            Self::Thursday => "Donnerstag",
            Self::Friday => "Freitag",
            Self::Saturday => "Samstag",
            Self::Sunday => "Sonntag",
        }
    }

    /// Returns the weekday name in English (e.g. "Monday", "Tuesday").
    pub fn english(&self) -> &'static str {
        match self {
            Self::Monday => "Monday",
            Self::Tuesday => "Tuesday",
            Self::Wednesday => "Wednesday",
            Self::Thursday => "Thursday",
            Self::Friday => "Friday",
            Self::Saturday => "Saturday",
            Self::Sunday => "Sunday",
        }
    }

    /// Returns the weekday name in French (e.g. "Lundi", "Mardi").
    pub fn french(&self) -> &'static str {
        match self {
            Self::Monday => "Lundi",
            Self::Tuesday => "Mardi",
            Self::Wednesday => "Mercredi",
            Self::Thursday => "Jeudi",
            Self::Friday => "Vendredi",
            Self::Saturday => "Samedi",
            Self::Sunday => "Dimanche",
        }
    }

    /// Returns the weekday name in Spanish (e.g. "Lunes", "Martes").
    pub fn spanish(&self) -> &'static str {
        match self {
            Self::Monday => "Lunes",
            Self::Tuesday => "Martes",
            Self::Wednesday => "Miércoles",
            Self::Thursday => "Jueves",
            Self::Friday => "Viernes",
            Self::Saturday => "Sábado",
            Self::Sunday => "Domingo",
        }
    }

    /// Returns the weekday name in Italian (e.g. "Lunedì", "Martedì").
    pub fn italian(&self) -> &'static str {
        match self {
            Self::Monday => "Lunedì",
            Self::Tuesday => "Martedì",
            Self::Wednesday => "Mercoledì",
            Self::Thursday => "Giovedì",
            Self::Friday => "Venerdì",
            Self::Saturday => "Sabato",
            Self::Sunday => "Domenica",
        }
    }
}
