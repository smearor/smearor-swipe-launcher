use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::AccessPointInfo;

pub const TOPIC_SCAN_RESULTS: &str = "service.network.scan_results";

/// WLAN scan results message broadcast by the service after a scan request.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanResultsMessage {
    /// List of access points found, sorted by signal strength (strongest first).
    pub access_points: stabby::vec::Vec<AccessPointInfo>,
    /// Timestamp of the scan as ISO-8601 string.
    pub scan_time: stabby::string::String,
}

impl TypedMessage for ScanResultsMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_network_model::ScanResultsMessage");
}

impl MessageTopic for ScanResultsMessage {
    fn topic() -> &'static str {
        TOPIC_SCAN_RESULTS
    }
}

impl SharedMessage for ScanResultsMessage {
    fn topic(&self) -> &'static str {
        TOPIC_SCAN_RESULTS
    }
}
