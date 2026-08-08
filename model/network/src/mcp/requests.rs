use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `network_toggle_radio` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct NetworkToggleRadioArgs {
    /// The radio technology to toggle
    pub technology: String,
    /// Whether the radio should be enabled
    pub enabled: bool,
}

/// Arguments for the `network_connect_wifi` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct NetworkConnectWifiArgs {
    /// The SSID of the WLAN to connect to
    pub ssid: String,
    /// The password for the WLAN (optional for known networks)
    pub password: Option<String>,
}

/// Arguments for the `network_toggle_vpn` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct NetworkToggleVpnArgs {
    /// The VPN profile name or UUID
    pub profile_name: String,
    /// Whether the VPN should be active
    pub active: bool,
}
