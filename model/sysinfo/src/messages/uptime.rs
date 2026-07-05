use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::TOPIC_UPTIME;

/// Status message for uptime and load average.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct UptimeStatusMessage {
    /// Time since system boot in seconds.
    pub uptime_seconds: u64,
    /// 1-minute load average.
    pub load_average_1_minute: f32,
    /// 5-minute load average.
    pub load_average_5_minute: f32,
    /// 15-minute load average.
    pub load_average_15_minute: f32,
}

impl UptimeStatusMessage {
    /// Creates a new uptime status message.
    pub fn new(uptime_seconds: u64, load_average_1_minute: f32, load_average_5_minute: f32, load_average_15_minute: f32) -> Self {
        Self {
            uptime_seconds,
            load_average_1_minute,
            load_average_5_minute,
            load_average_15_minute,
        }
    }
}

impl TypedMessage for UptimeStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_sysinfo_model::UptimeStatusMessage");
}

impl MessageTopic for UptimeStatusMessage {
    fn topic() -> &'static str {
        TOPIC_UPTIME
    }
}

impl SharedMessage for UptimeStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_UPTIME
    }
}
