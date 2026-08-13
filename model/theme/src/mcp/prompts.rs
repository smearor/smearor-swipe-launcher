use smearor_model_mcp::UnknownPromptError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP prompts registered by the theme service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMcpPrompts {
    /// Guide for theme management: get current theme, set theme, list themes.
    ThemeGuide,
}

impl AsRef<str> for ThemeMcpPrompts {
    fn as_ref(&self) -> &str {
        match self {
            Self::ThemeGuide => "theme_guide",
        }
    }
}

impl FromStr for ThemeMcpPrompts {
    type Err = UnknownPromptError;

    fn from_str(prompt: &str) -> Result<Self, Self::Err> {
        match prompt {
            "theme_guide" => Ok(Self::ThemeGuide),
            _ => Err(UnknownPromptError::new(prompt)),
        }
    }
}

impl Display for ThemeMcpPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
