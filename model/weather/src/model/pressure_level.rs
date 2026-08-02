use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

/// Atmospheric pressure classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PressureLevel {
    /// Below 1000 hPa — low pressure system.
    Low,
    /// 1000–1020 hPa — normal pressure.
    Normal,
    /// Above 1020 hPa — high pressure, stable weather.
    High,
}

impl From<f32> for PressureLevel {
    fn from(hpa: f32) -> Self {
        match hpa {
            p if p < 1000.0 => Self::Low,
            p if p <= 1020.0 => Self::Normal,
            _ => Self::High,
        }
    }
}

impl AsRef<str> for PressureLevel {
    fn as_ref(&self) -> &str {
        match self {
            Self::Low => "Tiefdruckgebiet",
            Self::Normal => "normaler Luftdruck",
            Self::High => "Hochdruckgebiet mit stabiler Wetterlage",
        }
    }
}

impl std::fmt::Display for PressureLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl WidgetIconRendering for PressureLevel {
    fn get_icon_color(&self) -> Option<Color> {
        let color = match self {
            Self::Low => Color::ORANGE,
            Self::Normal => Color::GREEN,
            Self::High => Color::LIGHT_GREEN,
        };
        Some(color)
    }

    fn get_icon_name(&self) -> Option<String> {
        Some(
            match self {
                Self::Low => "nf-md-gauge_empty",
                Self::Normal => "nf-md-gauge",
                Self::High => "nf-md-gauge_full",
            }
            .to_string(),
        )
    }
}
