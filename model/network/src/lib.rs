mod mcp;
mod messages;
mod model;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use mcp::resources::NetworkMcpResources;
pub use mcp::tools::NetworkMcpTools;
pub use messages::access_point::AccessPointInfo;
pub use messages::command::NetworkCommandAction;
pub use messages::command::NetworkCommandMessage;
pub use messages::command::TOPIC_COMMAND;
pub use messages::icon::network_interface_icon;
pub use messages::icon::network_interface_icon_unicode;
pub use messages::icon::wifi_security_icon;
pub use messages::icon::wifi_security_icon_unicode;
pub use messages::icon::wifi_signal_icon;
pub use messages::icon::wifi_signal_icon_unicode;
pub use messages::interface_status::InterfaceStatus;
pub use messages::scan_results::ScanResultsMessage;
pub use messages::scan_results::TOPIC_SCAN_RESULTS;
pub use messages::security::WifiSecurity;
pub use messages::state::NetworkConnectionState;
pub use messages::status::NetworkStatusMessage;
pub use messages::status::TOPIC_STATUS;
pub use messages::type_enum::NetworkInterfaceType;
pub use messages::view::NetworkView;
pub use messages::vpn_profiles::VpnProfileInfo;
pub use messages::vpn_profiles_message::TOPIC_VPN_PROFILES;
pub use messages::vpn_profiles_message::VpnProfilesMessage;
pub use model::ConnectionStateLevel;
pub use model::WifiSignalLevel;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(NetworkCommandMessageConverter, NetworkCommandMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(NetworkStatusMessageConverter, NetworkStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(ScanResultsMessageConverter, ScanResultsMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());

smearor_swipe_launcher_plugin_api::impl_json_convertible!(VpnProfilesMessageConverter, VpnProfilesMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());

/// Register all JSON converter implementations for network messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    NetworkCommandMessageConverter::register_in_host(context);
    NetworkStatusMessageConverter::register_in_host(context);
    ScanResultsMessageConverter::register_in_host(context);
    VpnProfilesMessageConverter::register_in_host(context);
}
