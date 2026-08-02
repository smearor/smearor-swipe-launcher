use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP resources exposed by the app-launcher service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppLauncherMcpResources {
    /// List of currently running applications.
    RunningApps,
    /// Paginated list of available applications.
    /// Supports query parameters `offset` and `limit`.
    AvailableApps,
}

impl AsRef<str> for AppLauncherMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::RunningApps => "app_launcher://running_apps",
            Self::AvailableApps => "app_launcher://available_apps",
        }
    }
}

impl FromStr for AppLauncherMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        let base = uri.split('?').next().unwrap_or(uri);
        match base {
            "app_launcher://running_apps" => Ok(Self::RunningApps),
            "app_launcher://available_apps" => Ok(Self::AvailableApps),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for AppLauncherMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
