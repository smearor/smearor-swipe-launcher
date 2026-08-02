use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the terminal-command service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalCommandMcpTools {
    /// Launch a configured terminal command.
    Launch,
    /// Terminate a running terminal command.
    Terminate,
    /// Restart a terminal command.
    Restart,
}

impl AsRef<str> for TerminalCommandMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::Launch => "terminal_command_launch",
            Self::Terminate => "terminal_command_terminate",
            Self::Restart => "terminal_command_restart",
        }
    }
}

impl FromStr for TerminalCommandMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "terminal_command_launch" => Ok(Self::Launch),
            "terminal_command_terminate" => Ok(Self::Terminate),
            "terminal_command_restart" => Ok(Self::Restart),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for TerminalCommandMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
