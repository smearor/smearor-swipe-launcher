use smearor_model_mcp::UnknownPromptError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP prompts registered by the voice assistant service.
///
/// Prompts not in this enum are considered external and are silently ignored,
/// as they are handled by the launcher core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceAssistantMcpPrompts {
    /// Current assistant state, transcript, and answer.
    VoiceAssistantStatus,
    /// Guide for using memory capabilities.
    MemoryGuide,
    /// Guide for discovering available MCP resources.
    ResourceDiscoveryGuide,
}

impl AsRef<str> for VoiceAssistantMcpPrompts {
    fn as_ref(&self) -> &str {
        match self {
            Self::VoiceAssistantStatus => "voice_assistant_status",
            Self::MemoryGuide => "memory_guide",
            Self::ResourceDiscoveryGuide => "resource_discovery_guide",
        }
    }
}

impl FromStr for VoiceAssistantMcpPrompts {
    type Err = UnknownPromptError;

    fn from_str(prompt: &str) -> Result<Self, Self::Err> {
        match prompt {
            "voice_assistant_status" => Ok(Self::VoiceAssistantStatus),
            "memory_guide" => Ok(Self::MemoryGuide),
            "resource_discovery_guide" => Ok(Self::ResourceDiscoveryGuide),
            _ => Err(UnknownPromptError::new(prompt)),
        }
    }
}

impl Display for VoiceAssistantMcpPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
