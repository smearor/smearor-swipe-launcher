use smearor_model_mcp::UnknownPromptError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP prompts registered by the personalization service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonalizationMcpPrompts {
    /// Guide for personalization queries, location, locale, and profile management.
    PersonalizationGuide,
}

impl AsRef<str> for PersonalizationMcpPrompts {
    fn as_ref(&self) -> &str {
        match self {
            Self::PersonalizationGuide => "personalization_guide",
        }
    }
}

impl FromStr for PersonalizationMcpPrompts {
    type Err = UnknownPromptError;

    fn from_str(prompt: &str) -> Result<Self, Self::Err> {
        match prompt {
            "personalization_guide" => Ok(Self::PersonalizationGuide),
            _ => Err(UnknownPromptError::new(prompt)),
        }
    }
}

impl Display for PersonalizationMcpPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
