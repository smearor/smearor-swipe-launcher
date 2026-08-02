use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP resources exposed by the power service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerMcpResources {
    /// Available power management capabilities (shutdown, reboot, suspend, etc.).
    Capabilities,
    /// Active power management inhibitors.
    Inhibitors,
    /// Currently scheduled power actions with remaining time.
    ScheduledActions,
}

impl AsRef<str> for PowerMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::Capabilities => "power://capabilities",
            Self::Inhibitors => "power://inhibitors",
            Self::ScheduledActions => "power://scheduled_actions",
        }
    }
}

impl FromStr for PowerMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri {
            "power://capabilities" => Ok(Self::Capabilities),
            "power://inhibitors" => Ok(Self::Inhibitors),
            "power://scheduled_actions" => Ok(Self::ScheduledActions),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for PowerMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
