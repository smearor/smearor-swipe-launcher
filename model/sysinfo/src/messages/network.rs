use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::TOPIC_NETWORK;

/// Status message for network metrics.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct NetworkStatusMessage {
    /// Aggregate inbound throughput in bytes per second.
    pub received_bytes_per_second: u64,
    /// Aggregate outbound throughput in bytes per second.
    pub transmitted_bytes_per_second: u64,
}

impl NetworkStatusMessage {
    /// Creates a new network status message.
    pub fn new(received_bytes_per_second: u64, transmitted_bytes_per_second: u64) -> Self {
        Self {
            received_bytes_per_second,
            transmitted_bytes_per_second,
        }
    }
}

impl TypedMessage for NetworkStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_sysinfo_model::NetworkStatusMessage");
}

impl MessageTopic for NetworkStatusMessage {
    fn topic() -> &'static str {
        TOPIC_NETWORK
    }
}

impl SharedMessage for NetworkStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_NETWORK
    }
}
