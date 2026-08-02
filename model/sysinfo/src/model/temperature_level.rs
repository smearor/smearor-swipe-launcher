use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

/// CPU temperature level with semantic coloring.
///
/// Maps a temperature in degrees Celsius to a color indicating
/// thermal severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SysinfoTemperatureLevel {
    /// Below 40°C — cool.
    Cool,
    /// 40-60°C — normal operating range.
    Normal,
    /// 60-75°C — warm, approaching throttle territory.
    Warm,
    /// 75-85°C — hot, likely throttling.
    Hot,
    /// Above 85°C — critical, potential damage risk.
    Critical,
}

impl SysinfoTemperatureLevel {
    /// Classifies a temperature in degrees Celsius into a thermal level.
    pub fn from_celsius(temp: f32) -> Self {
        match temp {
            t if t < 40.0 => Self::Cool,
            t if t < 60.0 => Self::Normal,
            t if t < 75.0 => Self::Warm,
            t if t < 85.0 => Self::Hot,
            _ => Self::Critical,
        }
    }
}

impl WidgetIconRendering for SysinfoTemperatureLevel {
    fn get_icon_color(&self) -> Option<Color> {
        let color = match self {
            Self::Cool => Color::BLUE,
            Self::Normal => Color::GREEN,
            Self::Warm => Color::YELLOW,
            Self::Hot => Color::ORANGE,
            Self::Critical => Color::RED,
        };
        Some(color)
    }

    fn get_icon_name(&self) -> Option<String> {
        let icon = match self {
            Self::Cool => "nf-fa-thermometer_empty",
            Self::Normal => "nf-fa-thermometer_quarter",
            Self::Warm => "nf-fa-thermometer_half",
            Self::Hot => "nf-fa-thermometer_full",
            Self::Critical => "nf-fa-thermometer_full",
        };
        Some(icon.to_string())
    }
}
