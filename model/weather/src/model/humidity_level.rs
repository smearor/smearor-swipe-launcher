use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

/// Relative humidity perception.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HumidityLevel {
    /// 0–30% — very dry air.
    VeryDry,
    /// 30–60% — comfortable humidity.
    Comfortable,
    /// 60–80% — high humidity.
    High,
    /// 80–100% — very muggy and wet air.
    Muggy,
}

impl From<f32> for HumidityLevel {
    fn from(percent: f32) -> Self {
        match percent {
            p if p < 30.0 => Self::VeryDry,
            p if p < 60.0 => Self::Comfortable,
            p if p < 80.0 => Self::High,
            _ => Self::Muggy,
        }
    }
}

impl AsRef<str> for HumidityLevel {
    fn as_ref(&self) -> &str {
        match self {
            Self::VeryDry => "sehr trockene Luft",
            Self::Comfortable => "angenehme Luftfeuchtigkeit",
            Self::High => "hohe Luftfeuchtigkeit",
            Self::Muggy => "sehr schwüle und feuchte Luft",
        }
    }
}

impl std::fmt::Display for HumidityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl WidgetIconRendering for HumidityLevel {
    fn get_icon_color(&self) -> Option<Color> {
        let color = match self {
            Self::VeryDry => Color::ORANGE,
            Self::Comfortable => Color::GREEN,
            Self::High => Color::YELLOW,
            Self::Muggy => Color::RED,
        };
        Some(color)
    }

    fn get_icon_name(&self) -> Option<String> {
        let name = match self {
            Self::VeryDry => "nf-md-water_outline",
            Self::Comfortable => "nf-md-water_check",
            Self::High => "nf-md-water",
            Self::Muggy => "nf-md-water_alert",
        };
        Some(name.to_string())
    }
}
