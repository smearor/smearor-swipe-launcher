use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the voice assistant service.
///
/// Tools not in this enum are considered external and are silently ignored,
/// as they are handled by the launcher core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceAssistantMcpTools {
    /// Activate the voice assistant.
    Activate,
    /// Deactivate the voice assistant.
    Deactivate,
    /// Submit text input to the voice assistant.
    SubmitText,
    /// Query the entity store by name or tool.
    MemoryQuery,
    /// Store a fact in semantic memory.
    MemoryStore,
    /// Recall facts from semantic memory by query.
    MemoryRecall,
    /// List keys in semantic memory, optionally filtered by category.
    MemoryList,
    /// Forget a fact from semantic memory by key.
    MemoryForget,
    /// Store a batch of facts in semantic memory.
    MemoryStoreBatch,
    /// Start training mode with an optional label.
    TrainingStart,
    /// End training mode and finalize the active trace.
    TrainingEnd,
    /// Get training traces by ID or query.
    TrainingGet,
    /// Switch the LLM model at runtime.
    SwitchModel,
    /// Set the tool selection threshold.
    SetThreshold,
    /// Set the rolling window keep_last parameter.
    SetRollingWindow,
    /// Set the maximum number of generation tokens.
    SetMaxTokens,
    /// Clear conversation history.
    ClearConversation,
    /// Get the current system prompt.
    GetSystemPrompt,
    /// Set a runtime system prompt override.
    SetSystemPrompt,
    /// Save the system prompt to a file.
    SaveSystemPrompt,
    /// Enable wake word detection.
    EnableWakeWord,
    /// Disable wake word detection.
    DisableWakeWord,
    /// Set the wake word model and optional threshold.
    SetWakeWordModel,
    /// Speak text directly via TTS, bypassing the LLM.
    Speak,
}

impl AsRef<str> for VoiceAssistantMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::Activate => "voice_assistant_activate",
            Self::Deactivate => "voice_assistant_deactivate",
            Self::SubmitText => "voice_assistant_submit_text",
            Self::MemoryQuery => "memory_query",
            Self::MemoryStore => "memory_store",
            Self::MemoryRecall => "memory_recall",
            Self::MemoryList => "memory_list",
            Self::MemoryForget => "memory_forget",
            Self::MemoryStoreBatch => "memory_store_batch",
            Self::TrainingStart => "voice_assistant_training_start",
            Self::TrainingEnd => "voice_assistant_training_end",
            Self::TrainingGet => "voice_assistant_training_get",
            Self::SwitchModel => "voice_assistant_switch_model",
            Self::SetThreshold => "voice_assistant_set_threshold",
            Self::SetRollingWindow => "voice_assistant_set_rolling_window",
            Self::SetMaxTokens => "voice_assistant_set_max_tokens",
            Self::ClearConversation => "voice_assistant_clear_conversation",
            Self::GetSystemPrompt => "voice_assistant_get_system_prompt",
            Self::SetSystemPrompt => "voice_assistant_set_system_prompt",
            Self::SaveSystemPrompt => "voice_assistant_save_system_prompt",
            Self::EnableWakeWord => "voice_assistant_enable_wake_word",
            Self::DisableWakeWord => "voice_assistant_disable_wake_word",
            Self::SetWakeWordModel => "voice_assistant_set_wake_word_model",
            Self::Speak => "voice_assistant_speak",
        }
    }
}

impl FromStr for VoiceAssistantMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "voice_assistant_activate" => Ok(Self::Activate),
            "voice_assistant_deactivate" => Ok(Self::Deactivate),
            "voice_assistant_submit_text" => Ok(Self::SubmitText),
            "memory_query" => Ok(Self::MemoryQuery),
            "memory_store" => Ok(Self::MemoryStore),
            "memory_recall" => Ok(Self::MemoryRecall),
            "memory_list" => Ok(Self::MemoryList),
            "memory_forget" => Ok(Self::MemoryForget),
            "memory_store_batch" => Ok(Self::MemoryStoreBatch),
            "voice_assistant_training_start" => Ok(Self::TrainingStart),
            "voice_assistant_training_end" => Ok(Self::TrainingEnd),
            "voice_assistant_training_get" => Ok(Self::TrainingGet),
            "voice_assistant_switch_model" => Ok(Self::SwitchModel),
            "voice_assistant_set_threshold" => Ok(Self::SetThreshold),
            "voice_assistant_set_rolling_window" => Ok(Self::SetRollingWindow),
            "voice_assistant_set_max_tokens" => Ok(Self::SetMaxTokens),
            "voice_assistant_clear_conversation" => Ok(Self::ClearConversation),
            "voice_assistant_get_system_prompt" => Ok(Self::GetSystemPrompt),
            "voice_assistant_set_system_prompt" => Ok(Self::SetSystemPrompt),
            "voice_assistant_save_system_prompt" => Ok(Self::SaveSystemPrompt),
            "voice_assistant_enable_wake_word" => Ok(Self::EnableWakeWord),
            "voice_assistant_disable_wake_word" => Ok(Self::DisableWakeWord),
            "voice_assistant_set_wake_word_model" => Ok(Self::SetWakeWordModel),
            "voice_assistant_speak" => Ok(Self::Speak),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for VoiceAssistantMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
