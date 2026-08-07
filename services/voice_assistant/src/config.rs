use crate::gpu_detection::get_available_vram;
use crate::gpu_detection::get_system_memory;
use crate::gpu_detection::has_discrete_gpu;
use crate::gpu_detection::vulkan_available;
use serde::Deserialize;
use smearor_voice_assistant_model::TtsConfig;
use smearor_voice_assistant_model::xdg_models_dir;
use std::str::FromStr;
use tracing::debug;

/// Default wake word detection threshold.
pub const DEFAULT_WAKE_WORD_THRESHOLD: f32 = 0.1;

/// Default grace period in milliseconds after VAD falling edge before exiting Listening Mode.
pub const DEFAULT_VAD_GRACE_PERIOD_MS: u64 = 400;

/// Default minimum continuous VAD activity in milliseconds before activating Listening Mode.
pub const DEFAULT_VAD_MIN_SPEECH_DURATION_MS: u64 = 100;

/// Default holdover time in milliseconds after TTS ends before re-enabling VAD edge detection.
pub const DEFAULT_TTS_MUTE_HOLDOVER_MS: u64 = 300;

/// Configuration for DoA hardware-VAD-triggered listening mode.
/// When enabled, the Voice Assistant uses the `speech_detected` flag from
/// `DoaStatusMessage` (broadcast by the DoA service) to activate and
/// deactivate the listening pipeline with near-zero latency.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DoaVadConfig {
    /// Whether DoA VAD-triggered listening mode is enabled.
    pub enabled: bool,
    /// Holdover time in milliseconds after VAD falling edge before exiting Listening Mode.
    pub grace_period_ms: u64,
    /// Minimum continuous VAD activity in milliseconds before activating Listening Mode.
    /// Prevents false triggers from impulsive environmental noises.
    pub min_speech_duration_ms: u64,
    /// If true, skip software wake-word detection when hardware VAD triggers.
    /// If false, wake-word detection is used as an additional confirmation criterion (Barge-In).
    pub skip_wake_word_on_vad: bool,
    /// Whether PipeWire AEC mirroring to XVF3800 is configured.
    /// When true, the software TTS-Mute-Window is disabled because the DSP handles echo cancellation.
    pub aec_mirroring_enabled: bool,
    /// Holdover time in milliseconds after TTS ends before re-enabling VAD edge detection.
    /// Only used when `aec_mirroring_enabled` is false.
    pub tts_mute_holdover_ms: u64,
}

impl Default for DoaVadConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            grace_period_ms: DEFAULT_VAD_GRACE_PERIOD_MS,
            min_speech_duration_ms: DEFAULT_VAD_MIN_SPEECH_DURATION_MS,
            skip_wake_word_on_vad: true,
            aec_mirroring_enabled: false,
            tts_mute_holdover_ms: DEFAULT_TTS_MUTE_HOLDOVER_MS,
        }
    }
}

/// Wake word model type for configuration.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub enum WakeWordModelType {
    /// Built-in Alexa model.
    #[default]
    Alexa,
    /// Built-in Hey Mycroft model.
    HeyMycroft,
    /// Custom ONNX model loaded from file.
    Custom,
}

impl FromStr for WakeWordModelType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "alexa" => Ok(WakeWordModelType::Alexa),
            "hey_mycroft" | "heymycroft" | "mycroft" => Ok(WakeWordModelType::HeyMycroft),
            "custom" => Ok(WakeWordModelType::Custom),
            _ => Err(format!("unknown wake word model '{s}', expected: Alexa, HeyMycroft, Custom")),
        }
    }
}

/// Configuration for wake word detection.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WakeWordServiceConfig {
    /// Whether wake word mode is enabled on startup.
    pub auto_enable: bool,
    /// Which wake word model to use.
    pub model: WakeWordModelType,
    /// Path to custom ONNX model file (only used when model is Custom).
    pub model_path: String,
    /// Detection threshold (0.0–1.0). Lower values are more sensitive.
    pub threshold: f32,
}

impl Default for WakeWordServiceConfig {
    fn default() -> Self {
        Self {
            auto_enable: false,
            model: WakeWordModelType::default(),
            model_path: String::new(),
            threshold: DEFAULT_WAKE_WORD_THRESHOLD,
        }
    }
}

/// Default speech probability threshold for VAD trimming.
pub const DEFAULT_VAD_THRESHOLD: f32 = 0.5;

pub const DEFAULT_THREADS: i32 = 4;

pub const DEFAULT_CONTEXT_SIZE: u32 = 4096;

pub const DEFAULT_BATCH_SIZE: u32 = 512;

/// Default max tokens for LLM generation. 512 is needed for tool-chaining with
/// larger models (e.g. Gemma 4 E4B) that produce longer JSON responses during
/// ReAct iterations.
pub const DEFAULT_MAX_TOKENS: usize = 512;

/// Default maximum tokens before context shifting is triggered.
pub const DEFAULT_MAX_CONTEXT_TOKENS: usize = 4096;

/// Default ratio of tokens to keep when shifting context (0.0–1.0).
pub const DEFAULT_CONTEXT_KEEP_RATIO: f64 = 0.8;

/// Default minimum tokens to preserve during context shifting.
pub const DEFAULT_MIN_PRESERVE_TOKENS: usize = 512;

/// Default number of trailing conversation messages to keep during rolling
/// window trimming. Each tool call/response pair is 2 messages, so 6 keeps
/// up to 3 tool-call/response pairs.
pub const DEFAULT_ROLLING_WINDOW_KEEP_LAST: usize = 6;

/// Estimate the model size in MB from the GGUF file on disk.
/// Falls back to a conservative default if the file cannot be read.
fn estimate_model_size_mb(model_path: &str) -> usize {
    match std::fs::metadata(model_path) {
        Ok(metadata) => (metadata.len() / (1024 * 1024)) as usize,
        Err(e) => {
            debug!("GPU auto-detection: cannot read model file '{}': {}, using default 2048 MB", model_path, e);
            2048
        }
    }
}

/// GPU device type classification.
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceType {
    /// Integrated GPU (shares system memory)
    IntegratedGpu,
    /// Discrete GPU (dedicated VRAM)
    DiscreteGpu,
    /// CPU-only (no GPU acceleration)
    Cpu,
}

/// GPU configuration for dynamic layer offloading.
#[derive(Debug, Clone)]
pub struct GpuConfig {
    /// GPU device type
    pub device_type: DeviceType,
    /// Available VRAM budget in MB
    pub vram_budget_mb: usize,
    /// Number of layers to offload to GPU (-1 = all layers)
    pub n_gpu_layers: i32,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            device_type: DeviceType::Cpu,
            vram_budget_mb: 0,
            n_gpu_layers: 0,
        }
    }
}

impl GpuConfig {
    /// Calculate optimal GPU layer offloading based on model size and available VRAM.
    pub fn calculate_optimal_layers(model_size_mb: usize, available_vram_mb: usize) -> i32 {
        // Reserve 512MB for system/overhead
        let usable_vram = available_vram_mb.saturating_sub(512);

        if usable_vram >= model_size_mb {
            // Full model fits on GPU
            -1 // All layers on GPU
        } else {
            // Calculate partial offloading
            let ratio = usable_vram as f64 / model_size_mb as f64;
            (ratio * 32.0).round() as i32 // Approximate layer count
        }
    }

    /// Detect optimal GPU configuration automatically.
    /// `model_path` is used to estimate the GGUF file size for accurate
    /// GPU layer offloading calculation instead of assuming a fixed size.
    ///
    /// The actual GPU backend (Vulkan, HIPBLAS, CPU) is selected at compile
    /// time via Cargo features. This function only detects device type and
    /// calculates VRAM budget and layer offloading count.
    pub fn detect_optimal_config(model_path: &str) -> Self {
        let model_size_mb = estimate_model_size_mb(model_path);
        debug!("GPU auto-detection: model size estimated at {} MB", model_size_mb);

        if vulkan_available() {
            if has_discrete_gpu() {
                let vram_mb = get_available_vram().saturating_sub(512);
                debug!("dGPU detected - {} MB VRAM available", vram_mb);
                GpuConfig {
                    device_type: DeviceType::DiscreteGpu,
                    vram_budget_mb: vram_mb,
                    n_gpu_layers: Self::calculate_optimal_layers(model_size_mb, vram_mb) as i32,
                }
            } else {
                let system_ram_mb = get_system_memory();
                let vram_mb = system_ram_mb / 4;
                debug!("iGPU detected - using shared system memory");
                GpuConfig {
                    device_type: DeviceType::IntegratedGpu,
                    vram_budget_mb: vram_mb,
                    n_gpu_layers: Self::calculate_optimal_layers(model_size_mb, vram_mb) as i32,
                }
            }
        } else {
            debug!("No GPU acceleration available - using CPU backend");
            GpuConfig {
                device_type: DeviceType::Cpu,
                vram_budget_mb: 0,
                n_gpu_layers: 0,
            }
        }
    }
}

/// Configuration for session management and context shifting.
///
/// Controls when the KV cache is shifted (instead of fully reset) and how
/// much context is preserved during rolling window trimming.
#[derive(Debug, Clone)]
pub struct ContextConfig {
    /// Maximum tokens before context shifting is triggered.
    pub max_context_tokens: usize,
    /// Ratio of tokens to keep when shifting (0.0–1.0).
    pub context_keep_ratio: f64,
    /// Minimum tokens to preserve (e.g. system prompt) during shifting.
    pub min_preserve_tokens: usize,
    /// Number of trailing conversation messages to always keep during rolling
    /// window trimming. Each tool call/response pair is 2 messages, so 6 keeps
    /// up to 3 tool-call/response pairs. The context message (index 0) is
    /// always preserved in addition to this count.
    pub rolling_window_keep_last: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: DEFAULT_MAX_CONTEXT_TOKENS,
            context_keep_ratio: DEFAULT_CONTEXT_KEEP_RATIO,
            min_preserve_tokens: DEFAULT_MIN_PRESERVE_TOKENS,
            rolling_window_keep_last: DEFAULT_ROLLING_WINDOW_KEEP_LAST,
        }
    }
}

/// Configuration for the LLM inference engine.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Path to the GGUF model file (e.g., "$XDG_DATA_HOME/smearor/models/qwen2.5-1.5b-instruct-q4_k_m.gguf").
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
    /// Fraction of n_ctx at which the session auto-resets.
    pub context_overflow_threshold: f32,
    /// GPU acceleration configuration
    pub gpu_config: GpuConfig,
    /// Session management and context shifting configuration.
    pub context_config: ContextConfig,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model_path: format!("{}/qwen2.5-1.5b-instruct-q4_k_m.gguf", xdg_models_dir()),
            n_threads: DEFAULT_THREADS,
            n_ctx: DEFAULT_CONTEXT_SIZE,
            n_batch: DEFAULT_BATCH_SIZE,
            max_tokens: DEFAULT_MAX_TOKENS,
            temperature: 0.7,
            top_k: 40,
            top_p: 0.95,
            context_overflow_threshold: 0.8,
            gpu_config: GpuConfig::default(),
            context_config: ContextConfig::default(),
        }
    }
}

/// Configuration for the voice assistant service.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct VoiceAssistantServiceConfig {
    /// Path to the Whisper GGML model file (e.g., "$XDG_DATA_HOME/smearor/models/ggml-tiny.bin").
    pub whisper_model_path: String,
    /// HuggingFace repo ID for auto-download of the Whisper model (optional).
    /// If empty, a hardcoded fallback mapping is used.
    #[serde(default)]
    pub whisper_model_repo: String,
    /// Path to the LLM GGUF model file (e.g., "$XDG_DATA_HOME/smearor/models/qwen2.5-1.5b-instruct-q4_k_m.gguf").
    pub llm_model_path: String,
    /// HuggingFace repo ID for auto-download of the LLM model (optional).
    /// If empty, a hardcoded fallback mapping is used.
    #[serde(default)]
    pub llm_model_repo: String,
    /// Number of CPU threads for LLM inference.
    pub llm_threads: u32,
    /// Maximum context window size in tokens for the LLM.
    pub llm_context_size: u32,
    ///
    pub llm_batch_size: u32,
    /// Maximum number of tokens to generate per LLM response.
    pub llm_max_tokens: usize,
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
    /// Maximum number of conversation messages to retain in short-term memory.
    pub max_history_messages: usize,
    /// Whether to inject entity states into the context message.
    pub inject_entity_states: bool,
    /// Path to the SQLite database file for long-term memory.
    pub memory_db_path: String,
    /// Maximum number of tools to inject into the context message after semantic selection.
    pub max_tools_in_prompt: usize,
    /// Maximum number of resources to inject into the context message after semantic selection.
    pub max_resources_in_prompt: usize,
    /// Maximum number of prompts to inject into the context message after semantic selection.
    pub max_prompts_in_prompt: usize,
    /// Whether to inject semantically recalled facts into the context message.
    pub inject_long_term_facts: bool,
    /// Number of facts to recall via fastembed for context message injection.
    pub max_recalled_facts: usize,
    /// Embedding model to use for semantic memory.
    pub embedding_model: String,
    /// Minimum cosine similarity for a tool to be included in the prompt.
    /// Tools below this threshold are excluded even if in top-N.
    pub tool_selection_threshold: f32,
    /// Fraction of n_ctx at which the session auto-resets.
    pub context_overflow_threshold: f32,
    /// Whether to enable MCP tool registration for this service.
    pub mcp_enabled: bool,
    /// GPU layer offloading (-1 = all layers, 0 = CPU only)
    pub n_gpu_layers: i32,
    /// Maximum tokens before context shifting is triggered.
    /// When the KV cache approaches this limit, oldest tokens are shifted out.
    pub max_context_tokens: usize,
    /// Ratio of tokens to keep when shifting context (0.0–1.0).
    /// E.g. 0.8 keeps the most recent 80% of tokens and removes the oldest 20%.
    pub context_keep_ratio: f64,
    /// Minimum tokens to preserve during context shifting (e.g. system prompt).
    /// The shifter will never remove tokens below this threshold.
    pub min_preserve_tokens: usize,
    /// Number of trailing conversation messages to always keep during rolling
    /// window trimming. Each tool call/response pair is 2 messages, so 6 keeps
    /// up to 3 tool-call/response pairs. The context message (index 0) is
    /// always preserved in addition to this count.
    pub rolling_window_keep_last: usize,
    /// Whether to use GBNF grammar enforcement to constrain LLM output to valid
    /// ReAct JSON format (`{"tool": ...}` or `{"final_answer": ...}`).
    /// Disable if the GPU backend crashes with grammar samplers.
    pub use_grammar: bool,
    /// Whether to enable Silero VAD post-processing to trim non-speech
    /// segments from captured audio before Whisper transcription.
    pub vad_enabled: bool,
    /// Path to the Silero VAD ONNX model file (e.g., "$XDG_DATA_HOME/smearor/models/silero_vad.onnx").
    pub vad_model_path: String,
    /// HuggingFace repo ID for auto-download of the VAD model (optional).
    /// If empty, a hardcoded fallback mapping is used.
    #[serde(default)]
    pub vad_model_repo: String,
    /// Speech probability threshold (0.0–1.0) for VAD trimming.
    /// Frames with probability below this value are considered non-speech.
    pub vad_threshold: f32,
    /// TTS (Text-to-Speech) configuration.
    pub tts: TtsConfig,
    /// Wake word detection configuration.
    pub wake_word: WakeWordServiceConfig,
    /// DoA hardware-VAD-triggered listening mode configuration.
    pub doa_vad: DoaVadConfig,
}

impl Default for VoiceAssistantServiceConfig {
    fn default() -> Self {
        Self {
            whisper_model_path: format!("{}/ggml-tiny.bin", xdg_models_dir()),
            whisper_model_repo: String::new(),
            llm_model_path: format!("{}/qwen2.5-1.5b-instruct-q4_k_m.gguf", xdg_models_dir()),
            llm_model_repo: String::new(),
            llm_threads: DEFAULT_THREADS as u32,
            llm_context_size: DEFAULT_CONTEXT_SIZE,
            llm_batch_size: DEFAULT_BATCH_SIZE,
            llm_max_tokens: DEFAULT_MAX_TOKENS,
            max_react_iterations: 8,
            llm_temperature: 0.1,
            audio_sample_rate: 16000,
            audio_channels: 1,
            max_recording_seconds: 10,
            silence_threshold_seconds: 1.5,
            language: "en".to_string(),
            auto_enable: false,
            max_catalog_chars: 4000,
            max_history_messages: 10,
            inject_entity_states: true,
            memory_db_path: "~/.local/share/smearor/memory.db".to_string(),
            max_tools_in_prompt: 20,
            max_resources_in_prompt: 10,
            max_prompts_in_prompt: 5,
            inject_long_term_facts: true,
            max_recalled_facts: 3,
            embedding_model: "bge-small-en-v1.5-q".to_string(),
            tool_selection_threshold: 0.3,
            context_overflow_threshold: 0.8,
            mcp_enabled: true,
            n_gpu_layers: -1,
            max_context_tokens: DEFAULT_MAX_CONTEXT_TOKENS,
            context_keep_ratio: DEFAULT_CONTEXT_KEEP_RATIO,
            min_preserve_tokens: DEFAULT_MIN_PRESERVE_TOKENS,
            rolling_window_keep_last: DEFAULT_ROLLING_WINDOW_KEEP_LAST,
            use_grammar: true,
            vad_enabled: true,
            vad_model_path: format!("{}/silero_vad.onnx", xdg_models_dir()),
            vad_model_repo: String::new(),
            vad_threshold: DEFAULT_VAD_THRESHOLD,
            tts: TtsConfig::default(),
            wake_word: WakeWordServiceConfig::default(),
            doa_vad: DoaVadConfig::default(),
        }
    }
}

impl VoiceAssistantServiceConfig {
    /// Builds an `LlmConfig` from the service configuration.
    pub fn to_llm_config(&self) -> LlmConfig {
        // GPU backend is selected at compile time via Cargo features.
        // Auto-detect device type and VRAM budget for layer offloading.
        let mut gpu_config = GpuConfig::detect_optimal_config(&self.llm_model_path);

        // Always apply explicit n_gpu_layers override from config.
        // -1 means "all layers on GPU" and must be respected, not skipped.
        gpu_config.n_gpu_layers = self.n_gpu_layers;

        LlmConfig {
            model_path: self.llm_model_path.clone(),
            n_threads: self.llm_threads as i32,
            n_ctx: self.llm_context_size,
            n_batch: self.llm_batch_size,
            max_tokens: self.llm_max_tokens,
            temperature: self.llm_temperature,
            top_k: 40,
            top_p: 0.95,
            context_overflow_threshold: self.context_overflow_threshold,
            gpu_config,
            context_config: ContextConfig {
                max_context_tokens: self.max_context_tokens,
                context_keep_ratio: self.context_keep_ratio,
                min_preserve_tokens: self.min_preserve_tokens,
                rolling_window_keep_last: self.rolling_window_keep_last,
            },
        }
    }

    /// Builds an `LlmConfig` from the service configuration with a different model path.
    /// Used for runtime model switching via MCP tool.
    /// When `n_ctx_override` or `max_tokens_override` are provided, they replace the
    /// config defaults — useful when a larger model needs a smaller context window
    /// to fit the KV cache in VRAM.
    pub fn to_llm_config_with_model(&self, model_path: &str, n_ctx_override: Option<u32>, max_tokens_override: Option<usize>) -> LlmConfig {
        let mut gpu_config = GpuConfig::detect_optimal_config(model_path);
        gpu_config.n_gpu_layers = self.n_gpu_layers;

        LlmConfig {
            model_path: model_path.to_string(),
            n_threads: self.llm_threads as i32,
            n_ctx: n_ctx_override.unwrap_or(self.llm_context_size),
            n_batch: self.llm_batch_size,
            max_tokens: max_tokens_override.unwrap_or(self.llm_max_tokens),
            temperature: self.llm_temperature,
            top_k: 40,
            top_p: 0.95,
            context_overflow_threshold: self.context_overflow_threshold,
            gpu_config,
            context_config: ContextConfig {
                max_context_tokens: self.max_context_tokens,
                context_keep_ratio: self.context_keep_ratio,
                min_preserve_tokens: self.min_preserve_tokens,
                rolling_window_keep_last: self.rolling_window_keep_last,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_TTS_MUTE_HOLDOVER_MS;
    use super::DEFAULT_VAD_GRACE_PERIOD_MS;
    use super::DEFAULT_VAD_MIN_SPEECH_DURATION_MS;
    use super::DoaVadConfig;

    #[test]
    fn test_doa_vad_config_default() {
        let config = DoaVadConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.grace_period_ms, DEFAULT_VAD_GRACE_PERIOD_MS);
        assert_eq!(config.min_speech_duration_ms, DEFAULT_VAD_MIN_SPEECH_DURATION_MS);
        assert!(config.skip_wake_word_on_vad);
        assert!(!config.aec_mirroring_enabled);
        assert_eq!(config.tts_mute_holdover_ms, DEFAULT_TTS_MUTE_HOLDOVER_MS);
    }

    #[test]
    fn test_doa_vad_config_serde_deserialize() {
        let json = serde_json::json!({
            "enabled": true,
            "grace_period_ms": 600,
            "min_speech_duration_ms": 150,
            "skip_wake_word_on_vad": false,
            "aec_mirroring_enabled": true,
            "tts_mute_holdover_ms": 500
        });
        let config: DoaVadConfig = serde_json::from_value(json).unwrap();
        assert!(config.enabled);
        assert_eq!(config.grace_period_ms, 600);
        assert_eq!(config.min_speech_duration_ms, 150);
        assert!(!config.skip_wake_word_on_vad);
        assert!(config.aec_mirroring_enabled);
        assert_eq!(config.tts_mute_holdover_ms, 500);
    }

    #[test]
    fn test_doa_vad_config_partial_json_uses_defaults() {
        let json = serde_json::json!({
            "enabled": true
        });
        let config: DoaVadConfig = serde_json::from_value(json).unwrap();
        assert!(config.enabled);
        assert_eq!(config.grace_period_ms, DEFAULT_VAD_GRACE_PERIOD_MS);
        assert_eq!(config.min_speech_duration_ms, DEFAULT_VAD_MIN_SPEECH_DURATION_MS);
        assert!(config.skip_wake_word_on_vad);
        assert!(!config.aec_mirroring_enabled);
        assert_eq!(config.tts_mute_holdover_ms, DEFAULT_TTS_MUTE_HOLDOVER_MS);
    }

    #[test]
    fn test_doa_vad_config_empty_json_uses_defaults() {
        let json = serde_json::json!({});
        let config: DoaVadConfig = serde_json::from_value(json).unwrap();
        assert!(!config.enabled);
        assert_eq!(config.grace_period_ms, DEFAULT_VAD_GRACE_PERIOD_MS);
    }

    #[test]
    fn test_vad_rising_edge_classification() {
        use smearor_doa_model::VadTransition;
        use smearor_doa_model::classify_vad_transition;
        assert_eq!(classify_vad_transition(false, true), VadTransition::RisingEdge);
    }

    #[test]
    fn test_vad_falling_edge_classification() {
        use smearor_doa_model::VadTransition;
        use smearor_doa_model::classify_vad_transition;
        assert_eq!(classify_vad_transition(true, false), VadTransition::FallingEdge);
    }

    #[test]
    fn test_vad_continuous_speech_classification() {
        use smearor_doa_model::VadTransition;
        use smearor_doa_model::classify_vad_transition;
        assert_eq!(classify_vad_transition(true, true), VadTransition::ContinuousSpeech);
    }

    #[test]
    fn test_vad_no_change_classification() {
        use smearor_doa_model::VadTransition;
        use smearor_doa_model::classify_vad_transition;
        assert_eq!(classify_vad_transition(false, false), VadTransition::NoChange);
    }

    #[test]
    fn test_vad_should_activate_after_min_duration() {
        use smearor_doa_model::should_activate_after_min_duration;
        let onset = std::time::Instant::now();
        let now = onset + std::time::Duration::from_millis(150);
        assert!(should_activate_after_min_duration(Some(onset), 100, now));
    }

    #[test]
    fn test_vad_should_not_activate_before_min_duration() {
        use smearor_doa_model::should_activate_after_min_duration;
        let onset = std::time::Instant::now();
        let now = onset + std::time::Duration::from_millis(50);
        assert!(!should_activate_after_min_duration(Some(onset), 100, now));
    }

    #[test]
    fn test_vad_should_not_activate_without_onset() {
        use smearor_doa_model::should_activate_after_min_duration;
        assert!(!should_activate_after_min_duration(None, 100, std::time::Instant::now()));
    }

    #[test]
    fn test_vad_should_activate_with_zero_min_duration() {
        use smearor_doa_model::should_activate_after_min_duration;
        let onset = std::time::Instant::now();
        assert!(should_activate_after_min_duration(Some(onset), 0, onset));
    }
}
