use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

/// Sky cloud coverage description.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudCoverLevel {
    /// 0–10% — crystal clear sky.
    Clear,
    /// 10–30% — fair and sunny.
    Fair,
    /// 30–60% — lightly clouded.
    PartlyCloudy,
    /// 60–85% — heavily clouded.
    MostlyCloudy,
    /// 85–100% — completely overcast.
    Overcast,
}

impl From<f32> for CloudCoverLevel {
    fn from(percent: f32) -> Self {
        match percent {
            p if p < 10.0 => Self::Clear,
            p if p < 30.0 => Self::Fair,
            p if p < 60.0 => Self::PartlyCloudy,
            p if p < 85.0 => Self::MostlyCloudy,
            _ => Self::Overcast,
        }
    }
}

impl AsRef<str> for CloudCoverLevel {
    fn as_ref(&self) -> &str {
        match self {
            Self::Clear => "strahlend klarer Himmel",
            Self::Fair => "heiter und sonnig",
            Self::PartlyCloudy => "leicht bewölkt",
            Self::MostlyCloudy => "stark bewölkt",
            Self::Overcast => "komplett bedeckt",
        }
    }
}

impl std::fmt::Display for CloudCoverLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl WidgetIconRendering for CloudCoverLevel {
    fn get_icon_color(&self) -> Option<Color> {
        None
    }

    fn get_icon_name(&self) -> Option<String> {
        let name = match self {
            Self::Clear => "nf-weather-day_sunny",
            Self::Fair => "nf-weather-day_sunny_overcast",
            Self::PartlyCloudy => "nf-weather-day_cloudy",
            Self::MostlyCloudy => "nf-weather-cloudy",
            Self::Overcast => "nf-weather-cloudy_gusts",
        };
        Some(name.to_string())
    }
}
