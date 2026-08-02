use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

/// Qualitative temperature perception in German.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemperatureLevel {
    /// Below 0°C — freezing frost.
    Freezing,
    /// 0–10°C — cold.
    Cold,
    /// 10–18°C — cool.
    Cool,
    /// 18–25°C — pleasantly mild.
    Pleasant,
    /// 25–30°C — warm.
    Warm,
    /// Above 30°C — hot.
    Hot,
}

impl From<f32> for TemperatureLevel {
    fn from(celsius: f32) -> Self {
        match celsius {
            t if t < 0.0 => Self::Freezing,
            t if t < 10.0 => Self::Cold,
            t if t < 18.0 => Self::Cool,
            t if t < 25.0 => Self::Pleasant,
            t if t < 30.0 => Self::Warm,
            _ => Self::Hot,
        }
    }
}

impl AsRef<str> for TemperatureLevel {
    fn as_ref(&self) -> &str {
        match self {
            Self::Freezing => "eiskalt mit Frost",
            Self::Cold => "kalt",
            Self::Cool => "kühl",
            Self::Pleasant => "angenehm mild",
            Self::Warm => "warm",
            Self::Hot => "heiß",
        }
    }
}

impl std::fmt::Display for TemperatureLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl WidgetIconRendering for TemperatureLevel {
    fn get_icon_color(&self) -> Option<Color> {
        let color = match self {
            Self::Freezing => Color::DARK_BLUE,
            Self::Cold => Color::BLUE,
            Self::Cool => Color::LIGHT_BLUE,
            Self::Pleasant => Color::YELLOW,
            Self::Warm => Color::ORANGE,
            Self::Hot => Color::RED,
        };
        Some(color)
    }

    fn get_icon_name(&self) -> Option<String> {
        Some(
            match self {
                Self::Freezing => "nf-fa-temperature_empty",
                Self::Cold => "nf-fa-temperature_quarter",
                Self::Cool => "nf-fa-temperature_half",
                Self::Pleasant => "nf-fa-temperature_three_quarters",
                Self::Warm => "nf-fa-temperature_full",
                Self::Hot => "nf-fa-temperature_high",
            }
            .to_string(),
        )
    }
}
