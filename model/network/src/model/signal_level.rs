use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

/// Wi-Fi signal strength level with semantic coloring.
///
/// Maps a signal percentage to a color indicating connection quality.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WifiSignalLevel {
    /// 0-25% — weak signal, unreliable connection.
    Weak,
    /// 25-50% — fair signal, may experience issues.
    Fair,
    /// 50-75% — good signal, stable connection.
    Good,
    /// 75-100% — excellent signal, optimal connection.
    Excellent,
}

impl WifiSignalLevel {
    /// Classifies a signal percentage into a signal level.
    pub fn from_percent(signal: u8) -> Self {
        match signal {
            s if s < 25 => Self::Weak,
            s if s < 50 => Self::Fair,
            s if s < 75 => Self::Good,
            _ => Self::Excellent,
        }
    }
}

impl WidgetIconRendering for WifiSignalLevel {
    fn get_icon_color(&self) -> Option<Color> {
        let color = match self {
            Self::Weak => Color::RED,
            Self::Fair => Color::ORANGE,
            Self::Good => Color::LIGHT_GREEN,
            Self::Excellent => Color::GREEN,
        };
        Some(color)
    }

    fn get_icon_name(&self) -> Option<String> {
        None
    }
}

/// Network connection state level with semantic coloring.
///
/// Maps a `NetworkConnectionState` to a color indicating connectivity status.
use crate::NetworkConnectionState;

/// Connection state level with semantic coloring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionStateLevel {
    /// Connected — fully operational.
    Connected,
    /// Connecting — in progress.
    Connecting,
    /// Disconnected — no connection.
    Disconnected,
    /// Failed — connection attempt failed.
    Failed,
    /// Unavailable — device not present or disabled.
    Unavailable,
}

impl ConnectionStateLevel {
    /// Creates a level from a `NetworkConnectionState`.
    pub fn from_state(state: NetworkConnectionState) -> Self {
        match state {
            NetworkConnectionState::Connected => Self::Connected,
            NetworkConnectionState::Connecting => Self::Connecting,
            NetworkConnectionState::Disconnected => Self::Disconnected,
            NetworkConnectionState::Failed => Self::Failed,
            NetworkConnectionState::Unavailable => Self::Unavailable,
        }
    }
}

impl WidgetIconRendering for ConnectionStateLevel {
    fn get_icon_color(&self) -> Option<Color> {
        let color = match self {
            Self::Connected => Color::GREEN,
            Self::Connecting => Color::YELLOW,
            Self::Disconnected => Color::ORANGE,
            Self::Failed => Color::RED,
            Self::Unavailable => Color::DARK_RED,
        };
        Some(color)
    }

    fn get_icon_name(&self) -> Option<String> {
        None
    }
}
