use serde::Deserialize;
use serde::Serialize;

/// A single ranking entry (tool, resource, or prompt) with its similarity score.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankingEntry {
    /// The name of the ranked item.
    pub name: String,
    /// The similarity score (0.0–1.0).
    pub score: f32,
}

/// Response for the `voice_assistant://status` resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatusResourceResponse {
    /// Current assistant state (e.g., "Idle", "Listening", "ThinkingLlm").
    pub state: String,
    /// Current transcript (partial recognition result).
    pub transcript: String,
    /// Current final answer (completed response).
    pub final_answer: String,
    /// Response type of the last answer (e.g., "final_answer", "clarify").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_type: Option<String>,
    /// Last tool ranking from the embedding-based tool selector.
    pub last_tool_ranking: Vec<RankingEntry>,
    /// Last resource ranking from the embedding-based resource selector.
    pub last_resource_ranking: Vec<RankingEntry>,
    /// Last prompt ranking from the embedding-based prompt selector.
    pub last_prompt_ranking: Vec<RankingEntry>,
}

/// A single tool catalog entry in the response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCatalogResponseEntry {
    /// Tool name (e.g., "system_power_action").
    pub name: String,
    /// Human-readable description of the tool.
    pub description: String,
    /// JSON schema for the tool's input parameters.
    pub input_schema: String,
}

/// Response for the `voice_assistant://tool_catalog` resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCatalogResourceResponse {
    /// All registered tools.
    pub tools: Vec<ToolCatalogResponseEntry>,
}

/// Response for the `voice_assistant://stt` resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SttResourceResponse {
    /// Path to the Whisper model file.
    pub whisper_model_path: String,
    /// Audio sample rate in Hz.
    pub audio_sample_rate: u32,
    /// Number of audio channels.
    pub audio_channels: u16,
    /// Maximum recording duration in seconds.
    pub max_recording_seconds: u32,
    /// Silence threshold in seconds for end-of-speech detection.
    pub silence_threshold_seconds: f32,
    /// BCP-47 language tag for recognition (e.g., "de", "en").
    pub language: String,
}

/// Response for the `voice_assistant://tts` resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TtsResourceResponse {
    /// Whether TTS is enabled at all.
    pub enabled: bool,
    /// Whether TTS is enabled for MCP text input path.
    pub tts_enabled_mcp: bool,
    /// Whether the LLM-based text-to-speech conversion step is active.
    pub conversion_step: bool,
    /// Whether espeak-ng phonemization is used before ONNX inference.
    pub phonemize_enabled: bool,
    /// Path to the ONNX model file.
    pub model_path: String,
    /// Path to the model config JSON file.
    pub config_path: String,
    /// TTS model type as string ("Piper" or "Kokoro").
    pub model_type: String,
    /// Native sample rate of the TTS model.
    pub model_sample_rate: u32,
    /// BCP-47 language tag for phonemization.
    pub language: String,
    /// Voice name (for Kokoro).
    pub voice: String,
}

/// Response for the `voice_assistant://embeddings` resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbeddingsResourceResponse {
    /// Name of the loaded embedding model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    /// Whether the engine was loaded as a fallback.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_fallback: Option<bool>,
    /// The configured embedding model identifier.
    pub configured_model: String,
    /// Number of entries in the embedding cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_entry_count: Option<u64>,
    /// Tool selection threshold (cosine similarity cutoff).
    pub tool_selection_threshold: f32,
    /// Error message if the embedding engine is not initialized.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// GGUF metadata extracted from a model file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GgufMetadataResponse {
    /// Model architecture (e.g., "llama", "qwen2").
    pub architecture: String,
    /// Human-readable model name.
    pub name: String,
    /// Maximum context length in tokens.
    pub context_length: String,
    /// Embedding dimension size.
    pub embedding_length: String,
    /// Number of transformer blocks (layers).
    pub block_count: String,
    /// Number of attention heads.
    pub head_count: String,
    /// Number of KV cache heads.
    pub head_count_kv: String,
    /// Quantization file type.
    pub file_type: String,
    /// Quantization version.
    pub quantization_version: String,
    /// Tokenizer model name.
    pub tokenizer_model: String,
    /// Total tensor count in the model.
    pub tensor_count: u64,
    /// GGUF version number.
    pub version: u32,
}

/// A single available model entry in the models resource response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelEntryResponse {
    /// Filename of the model file.
    pub filename: String,
    /// Full path to the model file.
    pub path: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// File size in megabytes.
    pub size_mb: f64,
    /// Extracted GGUF metadata (empty if parsing failed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GgufMetadataResponse>,
}

/// Response for the `voice_assistant://models` resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelsResourceResponse {
    /// Path to the currently active model.
    pub current_model: String,
    /// All available GGUF models in the models directory.
    pub available_models: Vec<ModelEntryResponse>,
}
