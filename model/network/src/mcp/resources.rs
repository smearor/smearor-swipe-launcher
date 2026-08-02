use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP resources exposed by the network service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkMcpResources {
    /// Current network interface status (primary interface, WiFi/WWAN state, throughput).
    Status,
    /// WiFi scan results (access points with SSID, signal, security).
    ScanResults,
    /// Configured VPN profiles and their active state.
    VpnProfiles,
}

impl AsRef<str> for NetworkMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::Status => "network://status",
            Self::ScanResults => "network://scan-results",
            Self::VpnProfiles => "network://vpn-profiles",
        }
    }
}

impl FromStr for NetworkMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri {
            "network://status" => Ok(Self::Status),
            "network://scan-results" => Ok(Self::ScanResults),
            "network://vpn-profiles" => Ok(Self::VpnProfiles),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for NetworkMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
