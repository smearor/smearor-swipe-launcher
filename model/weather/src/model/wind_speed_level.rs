use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

/// Wind speed classification based on the Beaufort scale (input in km/h).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindSpeedLevel {
    /// 0–2 km/h — calm.
    Calm,
    /// 2–12 km/h — light air.
    LightAir,
    /// 12–29 km/h — moderate breeze.
    ModerateBreeze,
    /// 29–50 km/h — fresh, noticeable wind.
    Fresh,
    /// 50–75 km/h — strong wind with gusts.
    Strong,
    /// 75–100 km/h — heavy storm.
    Storm,
    /// Above 100 km/h — hurricane force.
    Hurricane,
}

impl From<f32> for WindSpeedLevel {
    fn from(kmh: f32) -> Self {
        match kmh {
            w if w < 2.0 => Self::Calm,
            w if w < 12.0 => Self::LightAir,
            w if w < 29.0 => Self::ModerateBreeze,
            w if w < 50.0 => Self::Fresh,
            w if w < 75.0 => Self::Strong,
            w if w < 100.0 => Self::Storm,
            _ => Self::Hurricane,
        }
    }
}

impl AsRef<str> for WindSpeedLevel {
    fn as_ref(&self) -> &str {
        match self {
            Self::Calm => "windstill",
            Self::LightAir => "ein leiser Windzug",
            Self::ModerateBreeze => "eine mäßige Brise",
            Self::Fresh => "frischer, spürbarer Wind",
            Self::Strong => "starker Wind mit Sturmböen",
            Self::Storm => "schwerer Sturm",
            Self::Hurricane => "Orkanböen",
        }
    }
}

impl std::fmt::Display for WindSpeedLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl WidgetIconRendering for WindSpeedLevel {
    fn get_icon_color(&self) -> Option<Color> {
        let color = match self {
            Self::Calm => Color::GREEN,
            Self::LightAir => Color::LIGHT_GREEN,
            Self::ModerateBreeze => Color::YELLOW,
            Self::Fresh => Color::ORANGE,
            Self::Strong => Color::RED,
            Self::Storm => Color::DARK_RED,
            Self::Hurricane => Color::DARK_RED,
        };
        Some(color)
    }

    fn get_icon_name(&self) -> Option<String> {
        Some(
            match self {
                WindSpeedLevel::Calm => "nf-fa-circle_dot",
                WindSpeedLevel::LightAir => "nf-weather-windy",
                WindSpeedLevel::ModerateBreeze => "nf-weather-strong_wind",
                WindSpeedLevel::Fresh => "nf-weather-day_windy",
                WindSpeedLevel::Strong => "nf-weather-gale_warning",
                WindSpeedLevel::Storm => "nf-weather-storm_warning",
                WindSpeedLevel::Hurricane => "nf-weather-hurricane",
            }
            .to_string(),
        )
    }
}
