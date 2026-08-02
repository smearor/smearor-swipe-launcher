use crate::service::NetworkService;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

impl McpCapabilitiesRegistrator for NetworkService {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let status_resource = RegisterResourceMessage::new(
            "network://status",
            "Network Status",
            "Current network status including primary interface, SSID, signal, IP, and radio state.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(status_resource);

        let scan_resource = RegisterResourceMessage::new(
            "network://scan-results",
            "Network Scan Results",
            "List of all WLAN access points in range, including signal strength and encryption.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(scan_resource);

        let vpn_resource = RegisterResourceMessage::new(
            "network://vpn-profiles",
            "VPN Profiles",
            "List of all VPN connections registered in NetworkManager and their current state.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(vpn_resource);

        let toggle_radio_tool = RegisterToolMessage::new(
            "network_toggle_radio",
            "Toggles WLAN or airplane mode on/off.",
            r#"{ "type": "object", "properties": { "technology": { "type": "string", "enum": ["wifi", "wwan", "all"], "description": "The radio technology to toggle" }, "enabled": { "type": "boolean", "description": "Whether the radio should be enabled" } }, "required": ["technology", "enabled"] }"#,
        );
        broadcaster.broadcast_message_to_topic(toggle_radio_tool);

        let connect_wifi_tool = RegisterToolMessage::new(
            "network_connect_wifi",
            "Connects the system to a specific access point.",
            r#"{ "type": "object", "properties": { "ssid": { "type": "string", "description": "The SSID of the WLAN to connect to" }, "password": { "type": "string", "description": "The password for the WLAN (optional for known networks)" } }, "required": ["ssid"] }"#,
        );
        broadcaster.broadcast_message_to_topic(connect_wifi_tool);

        let toggle_vpn_tool = RegisterToolMessage::new(
            "network_toggle_vpn",
            "Starts or stops a specific VPN connection.",
            r#"{ "type": "object", "properties": { "profile_name": { "type": "string", "description": "The VPN profile name or UUID" }, "active": { "type": "boolean", "description": "Whether the VPN should be active" } }, "required": ["profile_name", "active"] }"#,
        );
        broadcaster.broadcast_message_to_topic(toggle_vpn_tool);

        let get_public_ip_tool = RegisterToolMessage::new(
            "network_get_public_ip",
            "Queries the external IP address and provider (GeoIP) via the internal HTTP service.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(get_public_ip_tool);
    }
}
