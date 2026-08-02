use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP resources exposed by the personalization service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonalizationMcpResources {
    /// User personalization profile (coordinates, timezone, locale).
    Profile,
}

impl AsRef<str> for PersonalizationMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::Profile => "personalization://profile",
        }
    }
}

impl FromStr for PersonalizationMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri {
            "personalization://profile" => Ok(Self::Profile),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for PersonalizationMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
