use crate::service::NetworkService;
use schemars::schema_for;
use smearor_model_mcp::NoArgs;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::ToolAnnotations;
use smearor_network_model::NetworkConnectWifiArgs;
use smearor_network_model::NetworkToggleRadioArgs;
use smearor_network_model::NetworkToggleVpnArgs;
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

        let toggle_radio_schema = serde_json::to_string(&schema_for!(NetworkToggleRadioArgs)).unwrap_or_default();
        let toggle_radio_tool = RegisterToolMessage::new("network_toggle_radio", "Toggles WLAN or airplane mode on/off.", &toggle_radio_schema)
            .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(toggle_radio_tool);

        let connect_wifi_schema = serde_json::to_string(&schema_for!(NetworkConnectWifiArgs)).unwrap_or_default();
        let connect_wifi_tool = RegisterToolMessage::new("network_connect_wifi", "Connects the system to a specific access point.", &connect_wifi_schema)
            .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(connect_wifi_tool);

        let toggle_vpn_schema = serde_json::to_string(&schema_for!(NetworkToggleVpnArgs)).unwrap_or_default();
        let toggle_vpn_tool = RegisterToolMessage::new("network_toggle_vpn", "Starts or stops a specific VPN connection.", &toggle_vpn_schema)
            .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(toggle_vpn_tool);

        let no_args_schema = serde_json::to_string(&schema_for!(NoArgs)).unwrap_or_default();
        let get_public_ip_tool = RegisterToolMessage::new(
            "network_get_public_ip",
            "Queries the external IP address and provider (GeoIP) via the internal HTTP service.",
            &no_args_schema,
        )
        .with_annotations(&ToolAnnotations::read_only().with_open_world(true));
        broadcaster.broadcast_message_to_topic(get_public_ip_tool);

        let prompt = RegisterPromptMessage::with_memory(
            "network_guide",
            "Returns a system prompt with network management tools, resources, and current status snapshot.",
            &no_args_schema,
            "network preferences including WiFi and VPN settings",
            "network,wifi,vpn,radio",
        );
        broadcaster.broadcast_message_to_topic(prompt);
    }
}
