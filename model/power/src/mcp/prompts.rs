use smearor_model_mcp::UnknownPromptError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP prompts registered by the power service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerMcpPrompts {
    /// Guide for executing power actions.
    PowerActionGuide,
    /// Safety guide for destructive power actions.
    PowerSafetyGuide,
}

impl AsRef<str> for PowerMcpPrompts {
    fn as_ref(&self) -> &str {
        match self {
            Self::PowerActionGuide => "power_action_guide",
            Self::PowerSafetyGuide => "power_safety_guide",
        }
    }
}

impl FromStr for PowerMcpPrompts {
    type Err = UnknownPromptError;

    fn from_str(prompt: &str) -> Result<Self, Self::Err> {
        match prompt {
            "power_action_guide" => Ok(Self::PowerActionGuide),
            "power_safety_guide" => Ok(Self::PowerSafetyGuide),
            _ => Err(UnknownPromptError::new(prompt)),
        }
    }
}

impl Display for PowerMcpPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
