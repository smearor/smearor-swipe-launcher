use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

/// Usage level for CPU, memory, and disk metrics.
///
/// Maps a percentage value to a semantic color indicating load severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsageLevel {
    /// 0-50% — low load.
    Low,
    /// 50-75% — moderate load.
    Moderate,
    /// 75-90% — high load.
    High,
    /// 90-100% — critical load.
    Critical,
}

impl UsageLevel {
    /// Classifies a percentage value into a usage level.
    pub fn from_percent(percent: f32) -> Self {
        match percent {
            p if p < 50.0 => Self::Low,
            p if p < 75.0 => Self::Moderate,
            p if p < 90.0 => Self::High,
            _ => Self::Critical,
        }
    }
}

impl WidgetIconRendering for UsageLevel {
    fn get_icon_color(&self) -> Option<Color> {
        let color = match self {
            Self::Low => Color::GREEN,
            Self::Moderate => Color::YELLOW,
            Self::High => Color::ORANGE,
            Self::Critical => Color::RED,
        };
        Some(color)
    }

    fn get_icon_name(&self) -> Option<String> {
        let icon = match self {
            Self::Low => "nf-md-gauge_empty",
            Self::Moderate => "nf-md-gauge_low",
            Self::High => "nf-md-gauge_full",
            Self::Critical => "nf-md-gauge_full",
        };
        Some(icon.to_string())
    }
}
