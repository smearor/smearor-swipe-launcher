use std::str::FromStr;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Locale {
    EnUs,
    DeDe,
    FrFr,
    ItIt,
    EsEs,
    #[default]
    Unknown,
}

impl FromStr for Locale {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_lowercase().replace('_', "-");
        if normalized.starts_with("en") {
            Ok(Locale::EnUs)
        } else if normalized.starts_with("de") {
            Ok(Locale::DeDe)
        } else if normalized.starts_with("fr") {
            Ok(Locale::FrFr)
        } else if normalized.starts_with("it") {
            Ok(Locale::ItIt)
        } else if normalized.starts_with("es") {
            Ok(Locale::EsEs)
        } else {
            Ok(Locale::Unknown)
        }
    }
}

/// Trait for types that can be derived from a locale string.
///
/// Implemented by personalization enums (e.g. `TemperatureUnit`, `TimeFormat`)
/// to provide locale-based default derivation (e.g. "en-US" → `Fahrenheit`).
pub trait FromLocale {
    /// Derives a value from a locale (e.g. "en-US", "de-DE").
    fn from_locale(locale: Locale) -> Self;
}
