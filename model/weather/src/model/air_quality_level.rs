use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

/// European Air Quality Index category (0–100+ scale).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AirQualityLevel {
    /// 0–20 — excellent air quality.
    Excellent,
    /// 20–40 — good air quality.
    Good,
    /// 40–60 — moderate air quality.
    Moderate,
    /// 60–80 — poor air quality for sensitive groups.
    Poor,
    /// Above 80 — very poor air quality.
    VeryPoor,
}

impl From<f32> for AirQualityLevel {
    fn from(aqi: f32) -> Self {
        match aqi {
            a if a < 20.0 => Self::Excellent,
            a if a < 40.0 => Self::Good,
            a if a < 60.0 => Self::Moderate,
            a if a < 80.0 => Self::Poor,
            _ => Self::VeryPoor,
        }
    }
}

impl AsRef<str> for AirQualityLevel {
    fn as_ref(&self) -> &str {
        match self {
            Self::Excellent => "ausgezeichnete Luftqualität",
            Self::Good => "gute Luftqualität",
            Self::Moderate => "mäßige Luftqualität",
            Self::Poor => "schlechte Luftqualität für empfindliche Personen",
            Self::VeryPoor => "sehr schlechte Luftbelastung",
        }
    }
}

impl std::fmt::Display for AirQualityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl WidgetIconRendering for AirQualityLevel {
    fn get_icon_color(&self) -> Option<Color> {
        let color = match self {
            Self::Excellent => Color::GREEN,
            Self::Good => Color::LIGHT_GREEN,
            Self::Moderate => Color::YELLOW,
            Self::Poor => Color::ORANGE,
            Self::VeryPoor => Color::RED,
        };
        Some(color)
    }

    fn get_icon_name(&self) -> Option<String> {
        Some(
            match self {
                AirQualityLevel::Excellent => "nf-fa-leaf",
                AirQualityLevel::Good => "nf-weather-fog",
                AirQualityLevel::Moderate => "nf-weather-smog",
                AirQualityLevel::Poor => "nf-fa-industry",
                AirQualityLevel::VeryPoor => "nf-fa-biohazard",
            }
            .to_string(),
        )
    }
}
