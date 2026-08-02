use smearor_model_mcp::UnknownPromptError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP prompts registered by the sysinfo service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SysinfoMcpPrompts {
    /// System health check guide with current snapshot.
    SystemHealthCheck,
}

impl AsRef<str> for SysinfoMcpPrompts {
    fn as_ref(&self) -> &str {
        match self {
            Self::SystemHealthCheck => "system_health_check",
        }
    }
}

impl FromStr for SysinfoMcpPrompts {
    type Err = UnknownPromptError;

    fn from_str(prompt: &str) -> Result<Self, Self::Err> {
        match prompt {
            "system_health_check" => Ok(Self::SystemHealthCheck),
            _ => Err(UnknownPromptError::new(prompt)),
        }
    }
}

impl Display for SysinfoMcpPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
