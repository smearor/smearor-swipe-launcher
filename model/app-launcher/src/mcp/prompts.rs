use smearor_model_mcp::UnknownPromptError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP prompts registered by the app-launcher service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppLauncherMcpPrompts {
    /// Guide for launching and managing applications.
    AppLaunchGuide,
}

impl AsRef<str> for AppLauncherMcpPrompts {
    fn as_ref(&self) -> &str {
        match self {
            Self::AppLaunchGuide => "app_launch_guide",
        }
    }
}

impl FromStr for AppLauncherMcpPrompts {
    type Err = UnknownPromptError;

    fn from_str(prompt: &str) -> Result<Self, Self::Err> {
        match prompt {
            "app_launch_guide" => Ok(Self::AppLaunchGuide),
            _ => Err(UnknownPromptError::new(prompt)),
        }
    }
}

impl Display for AppLauncherMcpPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
