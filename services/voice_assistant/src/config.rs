use serde::Deserialize;

/// Configuration for the LLM inference engine.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Path to the GGUF model file (e.g., "models/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf").
    pub model_path: String,
    /// Number of CPU threads for inference.
    pub n_threads: i32,
    /// Context window size in tokens.
    pub n_ctx: u32,
    /// Batch size for prompt processing.
    pub n_batch: u32,
    /// Maximum number of tokens to generate per response.
    pub max_tokens: usize,
    /// Sampling temperature (0.0 = greedy, 1.0 = creative).
    pub temperature: f32,
    /// Top-K sampling parameter.
    pub top_k: i32,
    /// Top-P (nucleus) sampling parameter.
    pub top_p: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model_path: "models/qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string(),
            n_threads: 4,
            n_ctx: 4096,
            n_batch: 2048,
            max_tokens: 256,
            temperature: 0.7,
            top_k: 40,
            top_p: 0.95,
        }
    }
}

/// Configuration for the voice assistant service.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct VoiceAssistantServiceConfig {
    /// Path to the Whisper GGML model file (e.g., "models/ggml-tiny.bin").
    pub whisper_model_path: String,
    /// Path to the LLM GGUF model file (e.g., "models/qwen2.5-1.5b-instruct-q4_k_m.gguf").
    pub llm_model_path: String,
    /// Number of CPU threads for LLM inference.
    pub llm_threads: u32,
    /// Maximum context window size in tokens for the LLM.
    pub llm_context_size: u32,
    /// Maximum number of ReAct loop iterations before giving up.
    pub max_react_iterations: u32,
    /// Sampling temperature for the LLM (0.0 = deterministic, 1.0 = creative).
    pub llm_temperature: f32,
    /// Audio sample rate for capture (Hz). Whisper expects 16000 Hz.
    pub audio_sample_rate: u32,
    /// Audio channels (1 = mono).
    pub audio_channels: u16,
    /// Maximum recording duration in seconds before auto-stopping.
    pub max_recording_seconds: u32,
    /// Silence detection threshold in seconds (stop recording after this much silence).
    pub silence_threshold_seconds: f32,
    /// System language for Whisper (e.g., "de" for German, "en" for English).
    pub language: String,
    /// Whether to enable the voice assistant on startup.
    pub auto_enable: bool,
    /// Maximum character budget for the tool catalog in the system prompt.
    pub max_catalog_chars: usize,
    /// System prompt template with {tools} placeholder for the tool catalog.
    /// If not set, a default prompt is used.
    pub system_prompt: Option<String>,
}

impl Default for VoiceAssistantServiceConfig {
    fn default() -> Self {
        Self {
            whisper_model_path: "models/ggml-tiny.bin".to_string(),
            llm_model_path: "models/qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string(),
            llm_threads: 4,
            llm_context_size: 4096,
            max_react_iterations: 8,
            llm_temperature: 0.1,
            audio_sample_rate: 16000,
            audio_channels: 1,
            max_recording_seconds: 30,
            silence_threshold_seconds: 1.5,
            language: "en".to_string(),
            auto_enable: false,
            max_catalog_chars: 4000,
            system_prompt: None,
        }
    }
}

impl VoiceAssistantServiceConfig {
    /// Builds an `LlmConfig` from the service configuration.
    pub fn to_llm_config(&self) -> LlmConfig {
        LlmConfig {
            model_path: self.llm_model_path.clone(),
            n_threads: self.llm_threads as i32,
            n_ctx: self.llm_context_size,
            n_batch: 2048,
            max_tokens: 256,
            temperature: self.llm_temperature,
            top_k: 40,
            top_p: 0.95,
        }
    }
}
