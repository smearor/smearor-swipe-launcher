use serde::Deserialize;
use typed_builder::TypedBuilder;

/// Configuration for the power service.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct PowerServiceConfig {
    /// Countdown duration in seconds before executing a power action.
    /// Set to 0 to disable the countdown and execute immediately.
    #[builder(default = 3)]
    pub countdown_seconds: u32,
    /// Whether to query inhibitors before showing the widget.
    #[builder(default = true)]
    pub enable_inhibitor_detection: bool,
    /// Whether to enable scheduled actions (sleep timer).
    #[builder(default = true)]
    pub enable_scheduled_actions: bool,
    /// Interval in seconds for refreshing capabilities and inhibitors.
    #[builder(default = 30)]
    pub refresh_interval_seconds: u64,
    /// Custom lock command (e.g., "hyprlock"). If empty, uses D-Bus session lock.
    #[builder(default = String::new())]
    pub lock_command: String,
    /// Custom logout command (e.g., "hyprctl dispatch exit"). If empty, uses D-Bus session terminate.
    #[builder(default = String::new())]
    pub logout_command: String,
}

impl Default for PowerServiceConfig {
    fn default() -> Self {
        Self {
            countdown_seconds: 3,
            enable_inhibitor_detection: true,
            enable_scheduled_actions: true,
            refresh_interval_seconds: 30,
            lock_command: String::new(),
            logout_command: String::new(),
        }
    }
}
