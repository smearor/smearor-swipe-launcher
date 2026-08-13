use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP resources registered by the theme service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMcpResources {
    /// Current theme status including applied theme and mode.
    Status,
    /// List of all configured themes with metadata.
    Themes,
}

impl AsRef<str> for ThemeMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::Status => "theme://status",
            Self::Themes => "theme://themes",
        }
    }
}

impl FromStr for ThemeMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri {
            "theme://status" => Ok(Self::Status),
            "theme://themes" => Ok(Self::Themes),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for ThemeMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
