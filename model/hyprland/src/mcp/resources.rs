use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP resources registered by the hyprland service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HyprlandMcpResources {
    /// Current Hyprland state (active window, fullscreen, keyboard layout).
    State,
    /// Active window information.
    ActiveWindow,
}

impl AsRef<str> for HyprlandMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::State => "hyprland://state",
            Self::ActiveWindow => "hyprland://active-window",
        }
    }
}

impl FromStr for HyprlandMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri {
            "hyprland://state" => Ok(Self::State),
            "hyprland://active-window" => Ok(Self::ActiveWindow),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for HyprlandMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
