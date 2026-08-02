use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

/// Precipitation intensity classified by mm per hour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrecipitationIntensity {
    /// No precipitation.
    Dry,
    /// 0–2.5 mm/h — light rain.
    Light,
    /// 2.5–10 mm/h — moderate rain.
    Moderate,
    /// 10–50 mm/h — heavy rain.
    Heavy,
    /// Above 50 mm/h — extreme downpour.
    Extreme,
}

impl From<f32> for PrecipitationIntensity {
    fn from(mm_per_hour: f32) -> Self {
        match mm_per_hour {
            n if n <= 0.0 => Self::Dry,
            n if n < 2.5 => Self::Light,
            n if n < 10.0 => Self::Moderate,
            n if n < 50.0 => Self::Heavy,
            _ => Self::Extreme,
        }
    }
}

impl AsRef<str> for PrecipitationIntensity {
    fn as_ref(&self) -> &str {
        match self {
            Self::Dry => "trocken",
            Self::Light => "leichter Regen",
            Self::Moderate => "mäßiger Regen",
            Self::Heavy => "starker Regen",
            Self::Extreme => "extremer Platzregen",
        }
    }
}

impl std::fmt::Display for PrecipitationIntensity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl WidgetIconRendering for PrecipitationIntensity {
    fn get_icon_color(&self) -> Option<Color> {
        None
    }

    fn get_icon_name(&self) -> Option<String> {
        let name = match self {
            Self::Dry => "nf-fa-hotjar",
            Self::Light => "nf-weather-sprinkle",
            Self::Moderate => "nf-weather-rain",
            Self::Heavy => "nf-weather-rain_wind",
            Self::Extreme => "nf-weather-storm_showers",
        };
        Some(name.to_string())
    }
}
