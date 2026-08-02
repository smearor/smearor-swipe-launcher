use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

use crate::BatteryStatus;

/// Battery charge level with semantic coloring.
///
/// Combines charge percentage and charging state to produce
/// a color indicating battery health.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryLevel {
    /// Below 15% — critical, needs immediate charging.
    Critical,
    /// 15-30% — low battery warning.
    Low,
    /// 30-60% — moderate charge.
    Moderate,
    /// 60-90% — high charge.
    High,
    /// Above 90% — nearly or fully charged.
    Full,
}

impl BatteryLevel {
    /// Classifies a battery percentage into a charge level.
    pub fn from_percent(percent: f32) -> Self {
        match percent {
            p if p < 15.0 => Self::Critical,
            p if p < 30.0 => Self::Low,
            p if p < 60.0 => Self::Moderate,
            p if p < 90.0 => Self::High,
            _ => Self::Full,
        }
    }

    /// Classifies a battery percentage, returning `Full` when charging status is `Full` or `Charging` with high charge.
    pub fn from_status(percent: f32, status: BatteryStatus) -> Self {
        if matches!(status, BatteryStatus::Full) {
            return Self::Full;
        }
        Self::from_percent(percent)
    }
}

impl WidgetIconRendering for BatteryLevel {
    fn get_icon_color(&self) -> Option<Color> {
        let color = match self {
            Self::Critical => Color::RED,
            Self::Low => Color::ORANGE,
            Self::Moderate => Color::YELLOW,
            Self::High => Color::LIGHT_GREEN,
            Self::Full => Color::GREEN,
        };
        Some(color)
    }

    fn get_icon_name(&self) -> Option<String> {
        None
    }
}
