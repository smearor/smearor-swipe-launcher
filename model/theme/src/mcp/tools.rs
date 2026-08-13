use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the theme service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMcpTools {
    /// Get the current theme status (applied theme, effective mode, configured themes).
    GetTheme,
    /// Set the current theme by name (selects and applies immediately).
    SetTheme,
}

impl AsRef<str> for ThemeMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::GetTheme => "get_theme",
            Self::SetTheme => "set_theme",
        }
    }
}

impl FromStr for ThemeMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "get_theme" => Ok(Self::GetTheme),
            "set_theme" => Ok(Self::SetTheme),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for ThemeMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
