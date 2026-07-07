use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::VpnProfileInfo;

pub const TOPIC_VPN_PROFILES: &str = "service.network.vpn_profiles";

/// VPN profiles message broadcast by the service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VpnProfilesMessage {
    /// List of all VPN connection profiles registered in NetworkManager.
    pub profiles: stabby::vec::Vec<VpnProfileInfo>,
    /// Timestamp of the last refresh as ISO-8601 string.
    pub last_updated: stabby::string::String,
}

impl TypedMessage for VpnProfilesMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_network_model::VpnProfilesMessage");
}

impl MessageTopic for VpnProfilesMessage {
    fn topic() -> &'static str {
        TOPIC_VPN_PROFILES
    }
}

impl SharedMessage for VpnProfilesMessage {
    fn topic(&self) -> &'static str {
        TOPIC_VPN_PROFILES
    }
}
