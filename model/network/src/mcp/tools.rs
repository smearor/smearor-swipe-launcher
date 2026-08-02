use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the network service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkMcpTools {
    /// Toggle a radio technology (wifi, bluetooth, etc.).
    ToggleRadio,
    /// Connect to a WiFi network.
    ConnectWifi,
    /// Toggle a VPN profile on/off.
    ToggleVpn,
    /// Query the public IP address.
    GetPublicIp,
}

impl AsRef<str> for NetworkMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::ToggleRadio => "network_toggle_radio",
            Self::ConnectWifi => "network_connect_wifi",
            Self::ToggleVpn => "network_toggle_vpn",
            Self::GetPublicIp => "network_get_public_ip",
        }
    }
}

impl FromStr for NetworkMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "network_toggle_radio" => Ok(Self::ToggleRadio),
            "network_connect_wifi" => Ok(Self::ConnectWifi),
            "network_toggle_vpn" => Ok(Self::ToggleVpn),
            "network_get_public_ip" => Ok(Self::GetPublicIp),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for NetworkMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
