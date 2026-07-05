use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::TOPIC_BATTERY;

/// Charging state of the battery.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub enum BatteryStatus {
    /// State is unknown.
    #[default]
    Unknown,
    /// Battery is discharging.
    Discharging,
    /// Battery is charging.
    Charging,
    /// Battery is fully charged.
    Full,
}

/// Status message for battery metrics.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct BatteryStatusMessage {
    /// Battery charge level in percent (0.0 - 100.0).
    pub level: f32,
    /// Current charging state.
    pub status: BatteryStatus,
}

impl BatteryStatusMessage {
    /// Creates a new battery status message.
    pub fn new(level: f32, status: BatteryStatus) -> Self {
        Self { level, status }
    }
}

impl TypedMessage for BatteryStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_sysinfo_model::BatteryStatusMessage");
}

impl MessageTopic for BatteryStatusMessage {
    fn topic() -> &'static str {
        TOPIC_BATTERY
    }
}

impl SharedMessage for BatteryStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_BATTERY
    }
}
