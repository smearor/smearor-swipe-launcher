use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

/// Sunshine duration relative to daylight duration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SunshineLevel {
    /// Sun barely visible (ratio < 0.1).
    BarelyVisible,
    /// Only a few hours of sun (ratio 0.1–0.4).
    FewHours(u32),
    /// Moderate sunshine (ratio 0.4–0.7).
    ModerateHours(u32),
    /// Very sunny day (ratio >= 0.7).
    VerySunny(u32),
}

impl SunshineLevel {
    /// Creates a sunshine level from sunshine and daylight durations in seconds.
    pub fn from_durations(sunshine_seconds: f32, daylight_seconds: f32) -> Self {
        let sun_hours = (sunshine_seconds / 3600.0).round() as u32;
        let ratio = if daylight_seconds > 0.0 { sunshine_seconds / daylight_seconds } else { 0.0 };
        match ratio {
            r if r < 0.1 => Self::BarelyVisible,
            r if r < 0.4 => Self::FewHours(sun_hours),
            r if r < 0.7 => Self::ModerateHours(sun_hours),
            _ => Self::VerySunny(sun_hours),
        }
    }
}

impl std::fmt::Display for SunshineLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BarelyVisible => write!(f, "heute zeigt sich die Sonne kaum"),
            Self::FewHours(hours) => write!(f, "heute gibt es nur etwa {hours} Stunden Sonne"),
            Self::ModerateHours(hours) => write!(f, "heute scheint für etwa {hours} Stunden die Sonne"),
            Self::VerySunny(hours) => write!(f, "ein sehr sonniger Tag mit rund {hours} Sonnenstunden"),
        }
    }
}

impl WidgetIconRendering for SunshineLevel {
    fn get_icon_color(&self) -> Option<Color> {
        Some(match self {
            Self::BarelyVisible => Color::BLACK,
            Self::FewHours(_) => Color::WHITE,
            Self::ModerateHours(_) => Color::YELLOW,
            Self::VerySunny(_) => Color::ORANGE,
        })
    }

    fn get_icon_name(&self) -> Option<String> {
        Some(
            match self {
                SunshineLevel::BarelyVisible => "nf-weather-cloudy",
                SunshineLevel::FewHours(_) => "nf-weather-day_cloudy_high",
                SunshineLevel::ModerateHours(_) => "nf-weather-day_sunny_overcast",
                SunshineLevel::VerySunny(_) => "nf-weather-day_sunny",
            }
            .to_string(),
        )
    }
}
