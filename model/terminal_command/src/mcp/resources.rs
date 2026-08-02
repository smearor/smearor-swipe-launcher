use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP resources exposed by the terminal-command service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalCommandMcpResources {
    /// Currently running terminal commands with their PIDs.
    Running,
    /// Configured terminal commands with their command lines and arguments.
    Configured,
}

impl AsRef<str> for TerminalCommandMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::Running => "terminal_command://running",
            Self::Configured => "terminal_command://configured",
        }
    }
}

impl FromStr for TerminalCommandMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri {
            "terminal_command://running" => Ok(Self::Running),
            "terminal_command://configured" => Ok(Self::Configured),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for TerminalCommandMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
