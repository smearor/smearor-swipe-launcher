use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::FromLocale;
use smearor_swipe_launcher_plugin_api::Locale;

/// The user's preferred date format.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DateFormat {
    /// Day.Month.Year (e.g. 26.07.2026) — common in Europe.
    #[default]
    Dmy,
    /// Month/Day/Year (e.g. 07/26/2026) — common in the US.
    Mdy,
    /// Year-Month-Day (e.g. 2026-07-26) — ISO 8601.
    Ymd,
}

impl std::str::FromStr for DateFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Dmy" => Ok(DateFormat::Dmy),
            "Mdy" => Ok(DateFormat::Mdy),
            "Ymd" => Ok(DateFormat::Ymd),
            _ => Err(format!("Unknown date format: {s}")),
        }
    }
}

impl FromLocale for DateFormat {
    fn from_locale(locale: Locale) -> Self {
        match locale {
            Locale::EnUs => DateFormat::Mdy,
            _ => DateFormat::Dmy,
        }
    }
}
