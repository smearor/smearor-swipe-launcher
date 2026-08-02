use smearor_model_mcp::UnknownPromptError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP prompts registered by the terminal-command service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalCommandMcpPrompts {
    /// Guide for launching and managing terminal commands.
    TerminalCommandGuide,
    /// Lifecycle guide for terminal command management.
    TerminalLifecycleGuide,
}

impl AsRef<str> for TerminalCommandMcpPrompts {
    fn as_ref(&self) -> &str {
        match self {
            Self::TerminalCommandGuide => "terminal_command_guide",
            Self::TerminalLifecycleGuide => "terminal_lifecycle_guide",
        }
    }
}

impl FromStr for TerminalCommandMcpPrompts {
    type Err = UnknownPromptError;

    fn from_str(prompt: &str) -> Result<Self, Self::Err> {
        match prompt {
            "terminal_command_guide" => Ok(Self::TerminalCommandGuide),
            "terminal_lifecycle_guide" => Ok(Self::TerminalLifecycleGuide),
            _ => Err(UnknownPromptError::new(prompt)),
        }
    }
}

impl Display for TerminalCommandMcpPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
