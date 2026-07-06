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
#[derive(Clone, Debug, Default, PartialEq)]
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
        countdown_action: PowerAction,
        last_updated: stabby::string::String,
    ) -> Self {
        Self {
            capabilities,
            inhibitors,
            scheduled_action,
            countdown_active,
            countdown_remaining_seconds,
            countdown_action,
            last_updated,
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
