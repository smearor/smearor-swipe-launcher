use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::FromLocale;
use smearor_swipe_launcher_plugin_api::Locale;
use std::str::FromStr;

/// The user's preferred time format.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeFormat {
    /// 24-hour format (e.g. 14:30).
    #[default]
    Hour24,
    /// 12-hour format with AM/PM (e.g. 2:30 PM).
    Hour12,
}

impl FromStr for TimeFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Hour24" | "24" => Ok(TimeFormat::Hour24),
            "Hour12" | "12" => Ok(TimeFormat::Hour12),
            _ => Err(format!("Unknown time format: {s}")),
        }
    }
}

impl FromLocale for TimeFormat {
    fn from_locale(locale: Locale) -> Self {
        match locale {
            Locale::EnUs => TimeFormat::Hour12,
            _ => Default::default(),
        }
    }
}
