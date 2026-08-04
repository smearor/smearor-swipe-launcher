use serde::Deserialize;
use serde::Serialize;
use smearor_personalization_model::TimeFormat;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::InhibitorInfo;
use crate::PowerAction;
use crate::PowerCapabilities;
use crate::ScheduledActionInfo;

pub const TOPIC_STATUS: &str = "service.power.status";

/// Complete power status message broadcast by the service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PowerStatusMessage {
    /// System capabilities as reported by systemd-logind.
    pub capabilities: PowerCapabilities,
    /// List of active inhibitor locks.
    pub inhibitors: stabby::vec::Vec<InhibitorInfo>,
    /// Currently scheduled action, if any.
    pub scheduled_action: stabby::option::Option<ScheduledActionInfo>,
    /// Whether a countdown is currently active for an immediate action.
    pub countdown_active: bool,
    /// Remaining seconds in the countdown (0 if no countdown is active).
    pub countdown_remaining_seconds: u32,
    /// Total seconds the countdown was set for.
    pub countdown_total_seconds: u32,
    /// The power action currently being counted down.
    pub countdown_action: PowerAction,
    /// Timestamp of the last status refresh as ISO-8601 string.
    pub last_updated: stabby::string::String,
}

impl PowerStatusMessage {
    /// Creates a new power status message.
    pub fn new(
        capabilities: PowerCapabilities,
        inhibitors: stabby::vec::Vec<InhibitorInfo>,
        scheduled_action: stabby::option::Option<ScheduledActionInfo>,
        countdown_active: bool,
        countdown_remaining_seconds: u32,
        countdown_total_seconds: u32,
        countdown_action: PowerAction,
        last_updated: stabby::string::String,
    ) -> Self {
        Self {
            capabilities,
            inhibitors,
            scheduled_action,
            countdown_active,
            countdown_remaining_seconds,
            countdown_total_seconds,
            countdown_action,
            last_updated,
        }
    }

    /// Returns a formatted countdown timer when an action is scheduled, or an empty string.
    ///
    /// Countdown localization is handled by the plugin via `PowerLabel::countdown_label`.
    pub fn scheduled_info_text(&self, time_format: TimeFormat) -> String {
        if let Some(sched) = self.scheduled_action.as_ref() {
            format_countdown(sched.remaining_seconds, time_format)
        } else {
            String::new()
        }
    }
}

/// Formats a countdown timer based on the preferred time format.
fn format_countdown(remaining_seconds: u64, time_format: TimeFormat) -> String {
    let hours = remaining_seconds / 3600;
    let minutes = (remaining_seconds % 3600) / 60;
    let seconds = remaining_seconds % 60;
    match time_format {
        TimeFormat::Hour24 => format!("{:02}:{:02}:{:02}", hours, minutes, seconds),
        TimeFormat::Hour12 => {
            if hours == 0 {
                format!("12:{:02}:{:02} AM", minutes, seconds)
            } else if hours < 12 {
                format!("{}:{:02}:{:02} AM", hours, minutes, seconds)
            } else if hours == 12 {
                format!("12:{:02}:{:02} PM", minutes, seconds)
            } else {
                format!("{}:{:02}:{:02} PM", hours - 12, minutes, seconds)
            }
        }
    }
}

impl TypedMessage for PowerStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_power_model::PowerStatusMessage");
}

impl MessageTopic for PowerStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl SharedMessage for PowerStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_STATUS
    }
}
