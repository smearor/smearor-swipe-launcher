use smearor_model_mcp::UnknownPromptError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP prompts registered by the DoA service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoaMcpPrompts {
    /// Guide for Direction of Arrival sensor queries and USB reconnection.
    DoaGuide,
}

impl AsRef<str> for DoaMcpPrompts {
    fn as_ref(&self) -> &str {
        match self {
            Self::DoaGuide => "doa_guide",
        }
    }
}

impl FromStr for DoaMcpPrompts {
    type Err = UnknownPromptError;

    fn from_str(prompt: &str) -> Result<Self, Self::Err> {
        match prompt {
            "doa_guide" => Ok(Self::DoaGuide),
            _ => Err(UnknownPromptError::new(prompt)),
        }
    }
}

impl Display for DoaMcpPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
