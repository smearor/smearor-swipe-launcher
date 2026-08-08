use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP resources registered by the GNOME/compositor service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositorMcpResources {
    /// Current workspace snapshot.
    Workspaces,
}

impl AsRef<str> for CompositorMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::Workspaces => "compositor://workspaces",
        }
    }
}

impl FromStr for CompositorMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri {
            "compositor://workspaces" => Ok(Self::Workspaces),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for CompositorMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
