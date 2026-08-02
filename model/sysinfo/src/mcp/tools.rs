use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the sysinfo service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SysinfoMcpTools {
    /// Refresh system information.
    Refresh,
}

impl AsRef<str> for SysinfoMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::Refresh => "sysinfo_refresh",
        }
    }
}

impl FromStr for SysinfoMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "sysinfo_refresh" => Ok(Self::Refresh),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for SysinfoMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
