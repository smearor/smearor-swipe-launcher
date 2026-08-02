use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP resources exposed by the voice assistant service.
///
/// Resources not in this enum are considered external and are silently ignored,
/// as they are handled by the launcher core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceAssistantMcpResources {
    /// Current assistant state, transcript, final answer, and rankings.
    Status,
    /// Registered tool catalog.
    ToolCatalog,
    /// Current LLM configuration (model path, context size, sampling parameters).
    Llm,
    /// Speech-to-text configuration.
    Stt,
    /// Text-to-speech configuration.
    Tts,
    /// Embedding engine configuration and tool selection threshold.
    Embeddings,
    /// Entity store contents from semantic memory.
    MemoryEntities,
    /// Available GGUF models in the models/ directory.
    Models,
}

impl AsRef<str> for VoiceAssistantMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::Status => "voice_assistant://status",
            Self::ToolCatalog => "voice_assistant://tool_catalog",
            Self::Llm => "voice_assistant://llm",
            Self::Stt => "voice_assistant://stt",
            Self::Tts => "voice_assistant://tts",
            Self::Embeddings => "voice_assistant://embeddings",
            Self::MemoryEntities => "memory://entities",
            Self::Models => "voice_assistant://models",
        }
    }
}

impl FromStr for VoiceAssistantMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri {
            "voice_assistant://status" => Ok(Self::Status),
            "voice_assistant://tool_catalog" => Ok(Self::ToolCatalog),
            "voice_assistant://llm" => Ok(Self::Llm),
            "voice_assistant://stt" => Ok(Self::Stt),
            "voice_assistant://tts" => Ok(Self::Tts),
            "voice_assistant://embeddings" => Ok(Self::Embeddings),
            "memory://entities" => Ok(Self::MemoryEntities),
            "voice_assistant://models" => Ok(Self::Models),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for VoiceAssistantMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
