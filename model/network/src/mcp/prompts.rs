use smearor_model_mcp::UnknownPromptError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP prompts registered by the network service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkMcpPrompts {
    /// Guide for network management: WiFi, VPN, radio, and IP queries.
    NetworkGuide,
}

impl AsRef<str> for NetworkMcpPrompts {
    fn as_ref(&self) -> &str {
        match self {
            Self::NetworkGuide => "network_guide",
        }
    }
}

impl FromStr for NetworkMcpPrompts {
    type Err = UnknownPromptError;

    fn from_str(prompt: &str) -> Result<Self, Self::Err> {
        match prompt {
            "network_guide" => Ok(Self::NetworkGuide),
            _ => Err(UnknownPromptError::new(prompt)),
        }
    }
}

impl Display for NetworkMcpPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
