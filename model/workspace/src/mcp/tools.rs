use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the GNOME/compositor service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositorMcpTools {
    /// Switch to a workspace by ID.
    SwitchWorkspace,
}

impl AsRef<str> for CompositorMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::SwitchWorkspace => "compositor_switch_workspace",
        }
    }
}

impl FromStr for CompositorMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "compositor_switch_workspace" => Ok(Self::SwitchWorkspace),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for CompositorMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
