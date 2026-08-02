use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::FromLocale;
use smearor_swipe_launcher_plugin_api::Locale;
use std::str::FromStr;

/// The user's preferred first day of the week.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirstDayOfWeek {
    /// Monday as the first day (default, ISO standard).
    #[default]
    Monday,
    /// Sunday as the first day (common in the US).
    Sunday,
}

impl FromStr for FirstDayOfWeek {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Monday" => Ok(FirstDayOfWeek::Monday),
            "Sunday" => Ok(FirstDayOfWeek::Sunday),
            _ => Err(format!("Unknown first day of week: {s}")),
        }
    }
}

impl FromLocale for FirstDayOfWeek {
    fn from_locale(locale: Locale) -> Self {
        match locale {
            Locale::EnUs => FirstDayOfWeek::Sunday,
            _ => Default::default(),
        }
    }
}
