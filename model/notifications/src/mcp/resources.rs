use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP resources registered by the notifications service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationMcpResources {
    /// Recent notification history.
    History,
    /// Current Do-Not-Disturb status.
    Dnd,
}

impl AsRef<str> for NotificationMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::History => "notifications://history",
            Self::Dnd => "notifications://dnd",
        }
    }
}

impl FromStr for NotificationMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri {
            "notifications://history" => Ok(Self::History),
            "notifications://dnd" => Ok(Self::Dnd),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for NotificationMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
