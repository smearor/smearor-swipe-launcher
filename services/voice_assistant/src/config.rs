use crate::gpu_detection::get_available_vram;
use crate::gpu_detection::get_system_memory;
use crate::gpu_detection::has_discrete_gpu;
use crate::gpu_detection::hipblas_libraries_available;
use crate::gpu_detection::is_amd_discrete_gpu;
use crate::gpu_detection::vulkan_available;
use serde::Deserialize;
use tracing::debug;

/// LLM backend selection for GPU acceleration.
#[derive(Debug, Clone, PartialEq)]
pub enum LlmBackend {
    /// Automatic detection (Vulkan > HIPBLAS > CPU)
    Auto,
    /// Vulkan compute backend (universal GPU support)
    Vulkan,
    /// AMD ROCm/HIPBLAS backend (maximum performance for AMD GPUs)
    Hipblas,
    /// CPU-only backend (fallback)
    Cpu,
}

impl Default for LlmBackend {
    fn default() -> Self {
        LlmBackend::Auto
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
    /// Selected backend for inference
    pub backend: LlmBackend,
    /// GPU device type
    pub device_type: DeviceType,
    /// Available VRAM budget in MB
    pub vram_budget_mb: usize,
    /// Number of layers to offload to GPU (-1 = all layers)
    pub n_gpu_layers: i32,
    /// Enable ROCm/HIPBLAS for AMD GPUs
    pub enable_hipblas: bool,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            backend: LlmBackend::Auto,
            device_type: DeviceType::Cpu,
            vram_budget_mb: 0,
            n_gpu_layers: 0,
            enable_hipblas: true,
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
    pub fn detect_optimal_config() -> Self {
        Self::detect_optimal_config_with_hipblas(true)
    }

    /// Detect optimal GPU configuration with HIPBLAS control.
    pub fn detect_optimal_config_with_hipblas(enable_hipblas: bool) -> Self {
        // GPU-Erkennung mit AMD-spezifischer Optimierung
        if vulkan_available() {
            // GPU-Typ erkennen und VRAM budgetieren
            if has_discrete_gpu() {
                let vram_mb = get_available_vram().saturating_sub(512); // 512MB Reserve

                // AMD dGPU: Prüfe ROCm/HIPBLAS Verfügbarkeit für maximale Performance
                if is_amd_discrete_gpu() && enable_hipblas && hipblas_libraries_available() {
                    debug!("AMD dGPU detected with ROCm/HIPBLAS support - using HIPBLAS backend");
                    GpuConfig {
                        backend: LlmBackend::Hipblas,
                        device_type: DeviceType::DiscreteGpu,
                        vram_budget_mb: vram_mb,
                        n_gpu_layers: Self::calculate_optimal_layers(2048, vram_mb) as i32,
                        enable_hipblas: true,
                    }
                } else {
                    debug!("dGPU detected but no ROCm/HIPBLAS or disabled - falling back to Vulkan");
                    GpuConfig {
                        backend: LlmBackend::Vulkan,
                        device_type: DeviceType::DiscreteGpu,
                        vram_budget_mb: vram_mb,
                        n_gpu_layers: Self::calculate_optimal_layers(2048, vram_mb) as i32,
                        enable_hipblas: false,
                    }
                }
            } else {
                // iGPU: Immer Vulkan (perfekte Universallösung)
                let system_ram_mb = get_system_memory();
                let vram_mb = system_ram_mb / 4;
                debug!("iGPU detected - using Vulkan backend");
                GpuConfig {
                    backend: LlmBackend::Vulkan,
                    device_type: DeviceType::IntegratedGpu,
                    vram_budget_mb: vram_mb,
                    n_gpu_layers: Self::calculate_optimal_layers(2048, vram_mb) as i32,
                    enable_hipblas: false,
                }
            }
        } else {
            // CPU Fallback
            debug!("No GPU acceleration available - using CPU backend");
            GpuConfig {
                backend: LlmBackend::Cpu,
                device_type: DeviceType::Cpu,
                vram_budget_mb: 0,
                n_gpu_layers: 0,
                enable_hipblas: false,
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
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 4096,
            context_keep_ratio: 0.8,
            min_preserve_tokens: 512,
        }
    }
}

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
            model_path: "models/qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string(),
            n_threads: 4,
            n_ctx: 4096,
            n_batch: 2048,
            max_tokens: 256,
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
    /// System prompt template (stable, no dynamic content).
    /// If not set, a default prompt is used.
    pub system_prompt: Option<String>,
    /// Maximum number of conversation messages to retain in short-term memory.
    pub max_history_messages: usize,
    /// Whether to inject entity states into the context message.
    pub inject_entity_states: bool,
    /// Path to the SQLite database file for long-term memory.
    pub memory_db_path: String,
    /// Maximum number of tools to inject into the context message after nucleo filtering.
    pub max_tools_in_prompt: usize,
    /// Whether to inject semantically recalled facts into the context message.
    pub inject_long_term_facts: bool,
    /// Number of facts to recall via fastembed for context message injection.
    pub max_recalled_facts: usize,
    /// Embedding model to use for semantic memory.
    pub embedding_model: String,
    /// Fraction of n_ctx at which the session auto-resets.
    pub context_overflow_threshold: f32,
    /// Whether to enable MCP tool registration for this service.
    pub mcp_enabled: bool,
    /// LLM backend selection (auto, vulkan, hipblas, cpu)
    pub llm_backend: String,
    /// Enable ROCm/HIPBLAS for AMD GPUs
    pub enable_hipblas: bool,
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
            max_history_messages: 10,
            inject_entity_states: true,
            memory_db_path: "~/.local/share/smearor/memory.db".to_string(),
            max_tools_in_prompt: 20,
            inject_long_term_facts: true,
            max_recalled_facts: 3,
            embedding_model: "bge-small-en-v1.5-q".to_string(),
            context_overflow_threshold: 0.8,
            mcp_enabled: true,
            llm_backend: "auto".to_string(),
            enable_hipblas: true,
            n_gpu_layers: -1,
            max_context_tokens: 4096,
            context_keep_ratio: 0.8,
            min_preserve_tokens: 512,
        }
    }
}

impl VoiceAssistantServiceConfig {
    /// Builds an `LlmConfig` from the service configuration.
    pub fn to_llm_config(&self) -> LlmConfig {
        // Parse backend string
        let backend = match self.llm_backend.as_str() {
            "auto" => LlmBackend::Auto,
            "vulkan" => LlmBackend::Vulkan,
            "hipblas" => LlmBackend::Hipblas,
            "cpu" => LlmBackend::Cpu,
            _ => {
                debug!("Unknown backend '{}', falling back to auto", self.llm_backend);
                LlmBackend::Auto
            }
        };

        // Create GPU configuration
        let mut gpu_config = if backend == LlmBackend::Auto {
            GpuConfig::detect_optimal_config_with_hipblas(self.enable_hipblas)
        } else {
            GpuConfig {
                backend: backend.clone(),
                device_type: DeviceType::Cpu, // Will be detected
                vram_budget_mb: 0,
                n_gpu_layers: self.n_gpu_layers,
                enable_hipblas: self.enable_hipblas,
            }
        };

        // Override n_gpu_layers if explicitly set
        if self.n_gpu_layers != -1 {
            gpu_config.n_gpu_layers = self.n_gpu_layers;
        }

        LlmConfig {
            model_path: self.llm_model_path.clone(),
            n_threads: self.llm_threads as i32,
            n_ctx: self.llm_context_size,
            n_batch: 2048,
            max_tokens: 256,
            temperature: self.llm_temperature,
            top_k: 40,
            top_p: 0.95,
            context_overflow_threshold: self.context_overflow_threshold,
            gpu_config,
            context_config: ContextConfig {
                max_context_tokens: self.max_context_tokens,
                context_keep_ratio: self.context_keep_ratio,
                min_preserve_tokens: self.min_preserve_tokens,
            },
        }
    }
}
