use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::FromLocale;
use smearor_swipe_launcher_plugin_api::Locale;
use std::str::FromStr;

/// The user's preferred measurement system.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeasurementSystem {
    /// Metric system (default).
    #[default]
    Metric,
    /// Imperial system.
    Imperial,
}

impl FromStr for MeasurementSystem {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Metric" => Ok(MeasurementSystem::Metric),
            "Imperial" => Ok(MeasurementSystem::Imperial),
            _ => Err(format!("Unknown measurement system: {s}")),
        }
    }
}

impl FromLocale for MeasurementSystem {
    fn from_locale(locale: Locale) -> Self {
        match locale {
            Locale::EnUs => MeasurementSystem::Imperial,
            _ => Default::default(),
        }
    }
}
