use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::FromLocale;
use smearor_swipe_launcher_plugin_api::Locale;
use std::str::FromStr;

/// The user's preferred temperature unit.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemperatureUnit {
    /// Degrees Celsius (default).
    #[default]
    Celsius,
    /// Degrees Fahrenheit.
    Fahrenheit,
}

impl FromStr for TemperatureUnit {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Celsius" | "C" => Ok(TemperatureUnit::Celsius),
            "Fahrenheit" | "F" => Ok(TemperatureUnit::Fahrenheit),
            _ => Err(format!("Unknown temperature unit: {s}")),
        }
    }
}

impl FromLocale for TemperatureUnit {
    fn from_locale(locale: Locale) -> Self {
        match locale {
            Locale::EnUs => TemperatureUnit::Fahrenheit,
            _ => Default::default(),
        }
    }
}
