use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

/// Precipitation probability category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrecipitationProbabilityLevel {
    /// 0–15% — rain almost impossible.
    Unlikely,
    /// 15–40% — low probability.
    Low,
    /// 40–70% — possible showers during the day.
    Possible,
    /// 70–100% — very high probability.
    VeryHigh,
}

impl From<f32> for PrecipitationProbabilityLevel {
    fn from(percent: f32) -> Self {
        match percent {
            p if p < 15.0 => Self::Unlikely,
            p if p < 40.0 => Self::Low,
            p if p < 70.0 => Self::Possible,
            _ => Self::VeryHigh,
        }
    }
}

impl AsRef<str> for PrecipitationProbabilityLevel {
    fn as_ref(&self) -> &str {
        match self {
            Self::Unlikely => "Regen ist heute fast ausgeschlossen",
            Self::Low => "geringe Regenwahrscheinlichkeit",
            Self::Possible => "mögliche Regenschauer im Tagesverlauf",
            Self::VeryHigh => "sehr hohe Regenwahrscheinlichkeit",
        }
    }
}

impl std::fmt::Display for PrecipitationProbabilityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl WidgetIconRendering for PrecipitationProbabilityLevel {
    fn get_icon_color(&self) -> Option<Color> {
        Some(match self {
            Self::Unlikely => Color::GREEN,
            Self::Low => Color::LIGHT_GREEN,
            Self::Possible => Color::YELLOW,
            Self::VeryHigh => Color::RED,
        })
    }

    fn get_icon_name(&self) -> Option<String> {
        Some(
            match self {
                Self::Unlikely => "nf-weather-na",
                Self::Low => "nf-weather-sprinkle",
                Self::Possible => "nf-weather-rain_mix",
                Self::VeryHigh => "nf-weather-storm_showers",
            }
            .to_string(),
        )
    }
}
