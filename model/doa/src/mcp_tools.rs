use std::fmt::Display;
use std::str::FromStr;

use smearor_model_mcp::UnknownResourceError;
use smearor_model_mcp::UnknownToolError;

/// MCP tools registered by the DoA service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoaMcpTools {
    /// Returns the current DoA angle, mapped direction, and connection status.
    GetDirection,
    /// Sets the DoA polling interval in milliseconds.
    SetPollInterval,
    /// Forces a USB reconnection to the ReSpeaker device.
    Reconnect,
}

impl AsRef<str> for DoaMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::GetDirection => "doa_get_direction",
            Self::SetPollInterval => "doa_set_poll_interval",
            Self::Reconnect => "doa_reconnect",
        }
    }
}

impl FromStr for DoaMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "doa_get_direction" => Ok(Self::GetDirection),
            "doa_set_poll_interval" => Ok(Self::SetPollInterval),
            "doa_reconnect" => Ok(Self::Reconnect),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for DoaMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

/// MCP resources registered by the DoA service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoaMcpResources {
    /// DoA sensor status resource URI.
    Status,
}

impl AsRef<str> for DoaMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::Status => "doa://status",
        }
    }
}

impl Display for DoaMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl FromStr for DoaMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri {
            "doa://status" => Ok(Self::Status),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}
