use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the app-launcher service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppLauncherMcpTools {
    /// Launch an application by desktop file path.
    Exec,
    /// Search available applications by name.
    SearchApps,
    /// Terminate a running application by desktop file path.
    Terminate,
}

impl AsRef<str> for AppLauncherMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::Exec => "app_launcher_exec",
            Self::SearchApps => "app_launcher_search_apps",
            Self::Terminate => "app_launcher_terminate",
        }
    }
}

impl FromStr for AppLauncherMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "app_launcher_exec" => Ok(Self::Exec),
            "app_launcher_search_apps" => Ok(Self::SearchApps),
            "app_launcher_terminate" => Ok(Self::Terminate),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for AppLauncherMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
