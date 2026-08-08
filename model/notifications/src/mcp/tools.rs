use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the notifications service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationMcpTools {
    /// Send a desktop notification.
    Send,
    /// Toggle Do-Not-Disturb mode.
    ToggleDnd,
    /// Dismiss all notifications.
    Clear,
}

impl AsRef<str> for NotificationMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::Send => "notifications_send",
            Self::ToggleDnd => "notifications_toggle_dnd",
            Self::Clear => "notifications_clear",
        }
    }
}

impl FromStr for NotificationMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "notifications_send" => Ok(Self::Send),
            "notifications_toggle_dnd" => Ok(Self::ToggleDnd),
            "notifications_clear" => Ok(Self::Clear),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for NotificationMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
