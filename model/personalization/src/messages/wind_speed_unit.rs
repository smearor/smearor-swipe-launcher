use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::FromLocale;
use smearor_swipe_launcher_plugin_api::Locale;
use std::str::FromStr;

/// The user's preferred wind speed unit.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindSpeedUnit {
    /// Kilometers per hour (default).
    #[default]
    Kmh,
    /// Miles per hour.
    Mph,
    /// Meters per second.
    Ms,
}

impl FromStr for WindSpeedUnit {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Kmh" | "kmh" => Ok(WindSpeedUnit::Kmh),
            "Mph" | "mph" => Ok(WindSpeedUnit::Mph),
            "Ms" | "ms" => Ok(WindSpeedUnit::Ms),
            _ => Err(format!("Unknown wind speed unit: {s}")),
        }
    }
}

impl FromLocale for WindSpeedUnit {
    fn from_locale(locale: Locale) -> Self {
        match locale {
            Locale::EnUs => WindSpeedUnit::Mph,
            _ => Default::default(),
        }
    }
}
