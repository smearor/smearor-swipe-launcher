Network management guide:

Tools:

- network_toggle_radio: Toggle WLAN or airplane mode on/off (technology: wifi/wwan/all, enabled: bool)
- network_connect_wifi: Connect to a specific access point (ssid, optional password for new networks)
- network_toggle_vpn: Start or stop a VPN connection (profile_name or UUID, active: bool)
- network_get_public_ip: Query the external IP address and provider via GeoIP

Resources:

- network://status: Current network status including primary interface, SSID, signal, IP, and radio state
- network://scan-results: List of all WLAN access points in range, including signal strength and encryption
- network://vpn-profiles: List of all VPN connections registered in NetworkManager and their current state

Notes:

- Always check network://status before making changes to understand the current state
- For WiFi connections, check scan-results first to find available SSIDs
- VPN profiles can be referenced by name or UUID
