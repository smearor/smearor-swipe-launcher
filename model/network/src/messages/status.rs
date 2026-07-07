use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::InterfaceStatus;

pub const TOPIC_STATUS: &str = "service.network.status";

/// Complete network status message broadcast by the service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetworkStatusMessage {
    /// Primary active interface (the one with internet access, or the first connected).
    pub primary_interface: InterfaceStatus,
    /// All active interfaces.
    pub interfaces: stabby::vec::Vec<InterfaceStatus>,
    /// Whether WLAN radio is enabled.
    pub wifi_enabled: bool,
    /// Whether WWAN (mobile broadband) radio is enabled.
    pub wwan_enabled: bool,
    /// Whether airplane mode is active (all radios off).
    pub airplane_mode: bool,
    /// Aggregate inbound throughput in bytes per second.
    pub received_bytes_per_second: u64,
    /// Aggregate outbound throughput in bytes per second.
    pub transmitted_bytes_per_second: u64,
    /// Timestamp of the last status refresh as ISO-8601 string.
    pub last_updated: stabby::string::String,
}

impl NetworkStatusMessage {
    /// Creates a new network status message.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        primary_interface: InterfaceStatus,
        interfaces: stabby::vec::Vec<InterfaceStatus>,
        wifi_enabled: bool,
        wwan_enabled: bool,
        airplane_mode: bool,
        received_bytes_per_second: u64,
        transmitted_bytes_per_second: u64,
        last_updated: stabby::string::String,
    ) -> Self {
        Self {
            primary_interface,
            interfaces,
            wifi_enabled,
            wwan_enabled,
            airplane_mode,
            received_bytes_per_second,
            transmitted_bytes_per_second,
            last_updated,
        }
    }
}

impl TypedMessage for NetworkStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_network_model::NetworkStatusMessage");
}

impl MessageTopic for NetworkStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl SharedMessage for NetworkStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_STATUS
    }
}
