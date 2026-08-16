use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use thiserror::Error;
use tracing::debug;
use tracing::warn;

use crate::config::ContextConfig;
use crate::config::LlmConfig;
use crate::llm_backend::local::LocalLlmBackend;
use crate::llm_backend::ollama::OllamaBackend;

/// The role of a chat message sender.
///
/// Serializes as the lowercase string representation (e.g. `"user"`, `"assistant"`,
/// `"system"`). The `Custom` variant serializes as its inner string.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ChatRole {
    /// The user sending a message to the assistant.
    User,
    /// The assistant responding to the user.
    Assistant,
    /// The system prompt providing instructions.
    System,
    /// A custom role not covered by the standard variants.
    Custom(String),
}

impl ChatRole {
    /// Returns the string representation of the role.
    pub fn as_str(&self) -> &str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
            Self::Custom(role) => role,
        }
    }
}

impl std::fmt::Display for ChatRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for ChatRole {
    fn from(role: &str) -> Self {
        match role {
            "user" => Self::User,
            "assistant" => Self::Assistant,
            "system" => Self::System,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl From<String> for ChatRole {
    fn from(role: String) -> Self {
        Self::from(role.as_str())
    }
}

/// Backend-agnostic chat message used throughout the ReAct loop and service layer.
///
/// `llama-cpp-4`'s `LlamaChatMessage` is confined to `LocalLlmBackend`'s
/// internal conversion logic. `OllamaBackend` serializes `ChatMessage`
/// directly to JSON — no `llama-cpp-4` types needed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message sender.
    pub role: ChatRole,
    /// The content text of the message.
    pub content: String,
}

impl ChatMessage {
    /// Creates a new chat message with the given role and content.
    pub fn new(role: &str, content: String) -> Self {
        Self {
            role: ChatRole::from(role),
            content,
        }
    }

    /// Creates a new user message.
    #[allow(dead_code)]
    pub fn user(content: String) -> Self {
        Self::new("user", content)
    }

    /// Creates a new assistant message.
    #[allow(dead_code)]
    pub fn assistant(content: String) -> Self {
        Self::new("assistant", content)
    }

    /// Creates a new system message.
    #[allow(dead_code)]
    pub fn system(content: String) -> Self {
        Self::new("system", content)
    }
}

/// The type of LLM backend to use for inference.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LlmBackendType {
    /// Local inference via llama.cpp (GGUF models).
    Local,
    /// Remote inference via Ollama HTTP API.
    Ollama,
}

impl Default for LlmBackendType {
    fn default() -> Self {
        Self::Local
    }
}

/// Errors that can occur during LLM inference across any backend.
#[derive(Debug, Error)]
pub enum LlmError {
    /// Failed to initialize the backend.
    #[error("Failed to initialize backend: {0}")]
    BackendInit(String),
    /// Failed to load the model.
    #[error("Failed to load model: {0}")]
    ModelLoad(String),
    /// Failed to create a context.
    #[error("Failed to create context: {0}")]
    ContextCreate(String),
    /// Failed to apply the chat template.
    #[error("Failed to apply chat template: {0}")]
    ApplyChatTemplate(String),
    /// Failed to tokenize the input.
    #[error("Failed to tokenize: {0}")]
    Tokenize(String),
    /// Failed to detokenize the output.
    #[error("Failed to detokenize: {0}")]
    Detokenize(String),
    /// Failed to decode a batch of tokens.
    #[error("Failed to decode batch: {0}")]
    Decode(String),
    /// Failed to create a chat message.
    #[error("Failed to create chat message: {0}")]
    ChatMessage(String),
    /// The generation exceeded the maximum token limit.
    #[error("Max tokens ({0}) reached")]
    MaxTokensReached(usize),
    /// The worker channel was closed.
    #[error("Worker channel closed")]
    ChannelClosed,
    /// An HTTP request to a remote backend timed out.
    #[error("Request timed out: {0}")]
    Timeout(String),
    /// An HTTP request to a remote backend failed.
    #[error("HTTP request failed: {0}")]
    Http(String),
    /// The requested model is not available on the remote backend.
    #[error("Model not available: {0}")]
    ModelNotAvailable(String),
    /// A JSON serialization or deserialization error.
    #[error("JSON error: {0}")]
    Json(String),
}

impl From<crate::llm::LlmError> for LlmError {
    fn from(error: crate::llm::LlmError) -> Self {
        match error {
            crate::llm::LlmError::BackendInit(msg) => Self::BackendInit(msg),
            crate::llm::LlmError::ModelLoad(msg) => Self::ModelLoad(msg),
            crate::llm::LlmError::ContextCreate(msg) => Self::ContextCreate(msg),
            crate::llm::LlmError::ApplyChatTemplate(msg) => Self::ApplyChatTemplate(msg),
            crate::llm::LlmError::Tokenize(msg) => Self::Tokenize(msg),
            crate::llm::LlmError::Detokenize(msg) => Self::Detokenize(msg),
            crate::llm::LlmError::Decode(msg) => Self::Decode(msg),
            crate::llm::LlmError::ChatMessage(msg) => Self::ChatMessage(msg),
            crate::llm::LlmError::MaxTokensReached(n) => Self::MaxTokensReached(n),
            crate::llm::LlmError::ChannelClosed => Self::ChannelClosed,
        }
    }
}

/// Configuration for the LLM backend, encompassing both local and remote settings.
#[derive(Clone, Debug)]
pub enum LlmBackendConfig {
    /// Configuration for local llama.cpp inference.
    Local(LlmConfig),
    /// Configuration for Ollama remote inference.
    Ollama(crate::config::OllamaConfig),
}

/// Resource report for the `voice_assistant://llm` MCP resource.
///
/// Serialized via serde to JSON. Fields are `Option<T>` to accommodate
/// backend-specific data — local backend fills local fields, Ollama fills
/// Ollama fields.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmResourceReport {
    /// The type of backend currently active.
    pub backend_type: String,
    /// Path to the GGUF model file (local backend only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_path: Option<String>,
    /// Context window size in tokens (local backend only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_ctx: Option<u32>,
    /// Batch size for prompt processing (local backend only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_batch: Option<u32>,
    /// Maximum number of tokens to generate per response.
    pub max_tokens: Option<usize>,
    /// Sampling temperature.
    pub temperature: Option<f32>,
    /// Top-K sampling parameter.
    pub top_k: Option<i32>,
    /// Top-P (nucleus) sampling parameter.
    pub top_p: Option<f32>,
    /// Number of CPU threads (local backend only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_threads: Option<i32>,
    /// Fraction of n_ctx at which the session auto-resets (local backend only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_overflow_threshold: Option<f32>,
    /// GPU layer offloading count (local backend only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_gpu_layers: Option<i32>,
    /// Rolling window keep_last parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rolling_window_keep_last: Option<usize>,
    /// Context keep ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_keep_ratio: Option<f64>,
    /// Minimum tokens to preserve during context shifting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_preserve_tokens: Option<usize>,
    /// Ollama server URL (Ollama backend only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ollama_url: Option<String>,
    /// Ollama model tag (Ollama backend only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ollama_model: Option<String>,
    /// Tool names invoked during the last ReAct loop execution.
    pub last_tool_calls: Vec<String>,
}

/// Trait abstracting LLM inference across local and remote backends.
///
/// Implementations:
/// - [`LocalLlmBackend`]: wraps `LlmWorker` (llama.cpp via `llama-cpp-4`)
/// - [`OllamaBackend`]: HTTP client to an Ollama server
///
/// The trait is async and `Send + Sync`, allowing it to be used behind
/// `ArcSwapOption<dyn LlmBackend>` for thread-safe runtime swapping.
#[async_trait]
pub trait LlmBackend: Send + Sync {
    /// Generates a completion from the system prompt and conversation history.
    ///
    /// Returns the generated text and the (possibly trimmed) conversation
    /// that was actually used for generation.
    async fn generate(
        &self,
        system_prompt: &str,
        conversation: Vec<ChatMessage>,
        max_tokens: usize,
        use_grammar: bool,
    ) -> Result<(String, Vec<ChatMessage>), LlmError>;

    /// Clears the conversation history (KV cache) while keeping model weights in memory.
    async fn clear_conversation(&self) -> Result<(), LlmError>;

    /// Trims the context to keep only the last `keep_last_n` tokens.
    async fn trim_context(&self, keep_last_n: usize) -> Result<(), LlmError>;

    /// Reloads the model from a new configuration at runtime.
    async fn reload_model(&self, config: LlmBackendConfig) -> Result<(), LlmError>;

    /// Updates the context configuration at runtime without reloading the model.
    async fn update_context_config(&self, context_config: ContextConfig) -> Result<(), LlmError>;

    /// Returns the current maximum number of tokens to generate per response.
    fn max_tokens(&self) -> usize;

    /// Updates `max_tokens` at runtime without reloading the model.
    fn set_max_tokens(&self, max_tokens: usize);

    /// Returns the type of this backend.
    #[allow(dead_code)]
    fn backend_type(&self) -> LlmBackendType;

    /// Builds a resource report for the `voice_assistant://llm` MCP resource.
    fn resource_report(&self, last_tool_calls: Vec<String>) -> LlmResourceReport;
}

/// Convenience helper: loads a backend from the given config.
pub fn create_backend(config: &LlmBackendConfig) -> Result<Arc<dyn LlmBackend>, LlmError> {
    match config {
        LlmBackendConfig::Local(llm_config) => {
            debug!("LLM backend: creating LocalLlmBackend");
            let engine = crate::llm::LlmInferenceEngine::load(llm_config).map_err(LlmError::from)?;
            let worker = crate::llm::LlmWorker::spawn(engine);
            Ok(Arc::new(LocalLlmBackend::new(worker)))
        }
        LlmBackendConfig::Ollama(ollama_config) => {
            debug!("LLM backend: creating OllamaBackend");
            let backend = OllamaBackend::new(ollama_config.clone()).map_err(LlmError::from)?;
            Ok(Arc::new(backend))
        }
    }
}

/// Converts a `LlmConfig` to a `LlmBackendConfig`.
impl From<LlmConfig> for LlmBackendConfig {
    fn from(config: LlmConfig) -> Self {
        Self::Local(config)
    }
}

/// Converts an `OllamaConfig` to a `LlmBackendConfig`.
impl From<crate::config::OllamaConfig> for LlmBackendConfig {
    fn from(config: crate::config::OllamaConfig) -> Self {
        Self::Ollama(config)
    }
}

/// Logs a warning when a backend operation fails and the error is non-fatal.
#[allow(dead_code)]
pub fn log_backend_error(context: &str, error: &LlmError) {
    warn!("LLM backend error during {context}: {error}");
}
