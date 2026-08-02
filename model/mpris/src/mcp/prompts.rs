use smearor_model_mcp::UnknownPromptError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP prompts registered by the MPRIS service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MprisMcpPrompts {
    /// Guide for controlling media playback.
    MprisControlGuide,
}

impl AsRef<str> for MprisMcpPrompts {
    fn as_ref(&self) -> &str {
        match self {
            Self::MprisControlGuide => "mpris_control_guide",
        }
    }
}

impl FromStr for MprisMcpPrompts {
    type Err = UnknownPromptError;

    fn from_str(prompt: &str) -> Result<Self, Self::Err> {
        match prompt {
            "mpris_control_guide" => Ok(Self::MprisControlGuide),
            _ => Err(UnknownPromptError::new(prompt)),
        }
    }
}

impl Display for MprisMcpPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
