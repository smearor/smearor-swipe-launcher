use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `voice_assistant_submit_text` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct VoiceAssistantSubmitTextArgs {
    /// The text command to submit
    pub text: String,
}

/// Arguments for the `memory_query` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MemoryQueryArgs {
    /// Entity name or tool name to look up
    pub query: String,
}

/// Arguments for the `memory_store` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MemoryStoreArgs {
    /// Short key for the fact
    pub key: String,
    /// The fact content
    pub value: String,
    /// Category: fact, preference, or habit
    pub category: Option<String>,
}

/// A single fact in a batch store operation.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MemoryFactEntry {
    /// Short key for the fact
    pub key: String,
    /// The fact content
    pub value: String,
    /// Category: fact, preference, or habit
    pub category: Option<String>,
}

/// Arguments for the `memory_store_batch` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MemoryStoreBatchArgs {
    /// Array of facts to store
    pub facts: Vec<MemoryFactEntry>,
}

/// Arguments for the `memory_recall` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MemoryRecallArgs {
    /// Natural language query to find related facts
    pub query: String,
    /// Max number of facts to return (default: 3)
    pub limit: Option<i32>,
}

/// Arguments for the `memory_list` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MemoryListArgs {
    /// Optional category filter: fact, preference, or habit
    pub category: Option<String>,
}

/// Arguments for the `memory_forget` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MemoryForgetArgs {
    /// The key of the fact to delete
    pub key: String,
}

/// Arguments for the `voice_assistant_switch_model` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct VoiceAssistantSwitchModelArgs {
    /// Path to the new GGUF model file (e.g., 'models/qwen2.5-7b-instruct-q4_k_m.gguf')
    pub model_path: String,
    /// Override the context window size (e.g., 4096 for larger models with limited VRAM). Omit to use the configured default.
    pub n_ctx: Option<i32>,
    /// Override the max tokens to generate per response. Omit to use the default of 512.
    pub max_tokens: Option<i32>,
    /// When true, download the model from HuggingFace if it doesn't exist locally. Uses fallback_models.toml mapping. Default: false.
    pub ensure_model: Option<bool>,
}

/// Arguments for the `resource_discovery_guide` MCP prompt.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ResourceDiscoveryGuideArgs {
    /// Optional keyword to filter resources by category
    pub filter: Option<String>,
}

/// Arguments for the `voice_assistant_training_start` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct VoiceAssistantTrainingStartArgs {
    /// Optional label for the training trace (e.g. 'favorite_song_test')
    pub label: Option<String>,
}

/// Arguments for the `voice_assistant_training_get` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct VoiceAssistantTrainingGetArgs {
    /// Maximum number of traces to return (default: 1)
    pub limit: Option<i32>,
    /// Optional label to filter traces
    pub label: Option<String>,
    /// Optional substring to search in user_text
    pub query: Option<String>,
    /// Optional specific trace ID to retrieve
    pub trace_id: Option<String>,
}

/// Arguments for the `voice_assistant_set_threshold` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct VoiceAssistantSetThresholdArgs {
    /// Minimum cosine similarity (0.0–1.0). Default: 0.3. Lower = more tools, higher = fewer tools.
    pub threshold: Option<f32>,
}

/// Arguments for the `voice_assistant_set_rolling_window` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct VoiceAssistantSetRollingWindowArgs {
    /// Number of trailing messages to keep (default: 6, i.e. 3 tool-call/response pairs). Minimum: 2.
    pub keep_last: Option<i32>,
}

/// Arguments for the `voice_assistant_set_max_tokens` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct VoiceAssistantSetMaxTokensArgs {
    /// Maximum number of tokens to generate per LLM response (default: 512). Typical range: 256–2048.
    pub max_tokens: Option<i32>,
}

/// Arguments for the `voice_assistant_set_system_prompt` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct VoiceAssistantSetSystemPromptArgs {
    /// The full system prompt text. Pass empty string to clear the override.
    pub system_prompt: String,
}

/// Arguments for the `voice_assistant_save_system_prompt` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct VoiceAssistantSaveSystemPromptArgs {
    /// The full system prompt text to save to the file.
    pub system_prompt: String,
}

/// Arguments for the `voice_assistant_set_wake_word_model` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct VoiceAssistantSetWakeWordModelArgs {
    /// Wake word model name: Alexa, HeyMycroft, or Custom
    pub model: String,
    /// Detection threshold (0.0-1.0). Lower = more sensitive. Optional.
    pub threshold: Option<f32>,
}

/// Arguments for the `voice_assistant_speak` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct VoiceAssistantSpeakArgs {
    /// The text to speak via TTS
    pub text: String,
}
