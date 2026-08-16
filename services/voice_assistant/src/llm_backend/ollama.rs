use async_trait::async_trait;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use std::sync::RwLock;
use std::time::Duration;
use tracing::debug;
use tracing::warn;

use crate::config::ContextConfig;
use crate::config::OllamaConfig;

use super::ChatMessage;
use super::LlmBackend;
use super::LlmBackendConfig;
use super::LlmBackendType;
use super::LlmError;
use super::LlmResourceReport;

// ============================================================================
// Typed Ollama API request/response structs
// ============================================================================

/// A single chat message in the Ollama API format.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OllamaChatMessage {
    /// The role of the message sender (e.g. "system", "user", "assistant").
    pub role: String,
    /// The content text of the message.
    pub content: String,
}

impl From<&ChatMessage> for OllamaChatMessage {
    fn from(msg: &ChatMessage) -> Self {
        Self {
            role: msg.role.as_str().to_string(),
            content: msg.content.clone(),
        }
    }
}

/// Generation options passed to the Ollama API.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OllamaChatOptions {
    /// Maximum number of tokens to generate.
    pub num_predict: usize,
    /// Sampling temperature.
    pub temperature: f32,
    /// Top-K sampling parameter.
    pub top_k: i32,
    /// Top-P (nucleus) sampling parameter.
    pub top_p: f32,
}

/// The `format` field for Ollama's Structured Outputs feature.
///
/// When `None`, no format constraint is applied.
/// When `Schema`, a JSON Schema is passed to enforce structured output.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OllamaFormat {
    /// No format constraint (plain text generation).
    None(String),
    /// JSON Schema for structured output enforcement.
    Schema(serde_json::Value),
}

/// Request body for the Ollama `/api/chat` endpoint.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OllamaChatRequest {
    /// The model tag to use for generation (e.g. "gemma2:9b-instruct-q4_K_M").
    pub model: String,
    /// The conversation messages.
    pub messages: Vec<OllamaChatMessage>,
    /// Whether to stream the response (always false for our use case).
    pub stream: bool,
    /// Generation options.
    pub options: OllamaChatOptions,
    /// Output format constraint (JSON Schema or "json").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<OllamaFormat>,
}

/// The message field in an Ollama chat response.
#[derive(Clone, Debug, Deserialize)]
pub struct OllamaChatResponseMessage {
    /// The generated content text.
    pub content: String,
}

/// Response body from the Ollama `/api/chat` endpoint.
#[derive(Clone, Debug, Deserialize)]
pub struct OllamaChatResponse {
    /// The generated message.
    pub message: OllamaChatResponseMessage,
}

/// A single model entry in the Ollama `/api/tags` response.
#[derive(Clone, Debug, Deserialize)]
pub struct OllamaTagEntry {
    /// The model tag name (e.g. "gemma2:9b-instruct-q4_K_M").
    pub name: String,
}

/// Response body from the Ollama `/api/tags` endpoint.
#[derive(Clone, Debug, Deserialize)]
pub struct OllamaTagsResponse {
    /// List of available models.
    pub models: Vec<OllamaTagEntry>,
}

// ============================================================================
// ReAct JSON Schema (loaded from file via include_str!)
// ============================================================================

/// JSON Schema for ReAct structured output.
/// Passed to Ollama's `format` field when `use_grammar` is true.
/// Uses `oneOf` to enforce exactly one action type per response.
///
/// Loaded from `data/react_schema.json` via `include_str!` and parsed once
/// at startup.
static REACT_JSON_SCHEMA: Lazy<serde_json::Value> =
    Lazy::new(|| serde_json::from_str(include_str!("../../data/react_schema.json")).expect("react_schema.json must be valid JSON"));

// ============================================================================
// OllamaBackend
// ============================================================================

/// Remote LLM backend using the Ollama HTTP API.
///
/// Communicates with an Ollama server via `reqwest::Client`. All API
/// requests and responses use typed Serde structs — no manual JSON
/// construction.
pub struct OllamaBackend {
    /// The HTTP client with configured timeouts.
    client: Client,
    /// Ollama server configuration (URL, model, timeouts).
    config: RwLock<OllamaConfig>,
}

impl OllamaBackend {
    /// Creates a new Ollama backend with the given configuration.
    ///
    /// Builds a `reqwest::Client` with separate connect and request timeouts
    /// for interactive responsiveness.
    pub fn new(config: OllamaConfig) -> Result<Self, LlmError> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(config.connect_timeout_seconds))
            .timeout(Duration::from_secs(config.request_timeout_seconds))
            .build()
            .map_err(|error| LlmError::BackendInit(error.to_string()))?;

        debug!(
            "OllamaBackend: created client for {} (connect: {}s, request: {}s)",
            config.url, config.connect_timeout_seconds, config.request_timeout_seconds
        );

        Ok(Self {
            client,
            config: RwLock::new(config),
        })
    }

    /// Returns the current configuration.
    fn config(&self) -> OllamaConfig {
        self.config.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Checks whether the configured model is available on the Ollama server.
    async fn check_model_available(&self, model: &str) -> Result<bool, LlmError> {
        let url = format!("{}/api/tags", self.config().url);
        let response = self.client.get(&url).send().await.map_err(|error| LlmError::Http(error.to_string()))?;

        if !response.status().is_success() {
            return Err(LlmError::Http(format!("Ollama /api/tags returned {}", response.status())));
        }

        let tags_response: OllamaTagsResponse = response.json().await.map_err(|error| LlmError::Json(error.to_string()))?;

        let available = tags_response.models.iter().any(|entry| entry.name == model);
        if !available {
            warn!(
                "OllamaBackend: model '{}' not found on server. Available: {:?}",
                model,
                tags_response.models.iter().map(|m| &m.name).collect::<Vec<_>>()
            );
        }
        Ok(available)
    }
}

#[async_trait]
impl LlmBackend for OllamaBackend {
    async fn generate(
        &self,
        system_prompt: &str,
        conversation: Vec<ChatMessage>,
        max_tokens: usize,
        use_grammar: bool,
    ) -> Result<(String, Vec<ChatMessage>), LlmError> {
        let config = self.config();

        // Check model availability on first call or after model switch.
        if !self.check_model_available(&config.model).await? {
            return Err(LlmError::ModelNotAvailable(config.model.clone()));
        }

        // Build typed request.
        let mut messages = Vec::with_capacity(conversation.len() + 1);
        messages.push(OllamaChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        });
        for msg in &conversation {
            messages.push(OllamaChatMessage::from(msg));
        }

        let format = if use_grammar {
            Some(OllamaFormat::Schema(REACT_JSON_SCHEMA.clone()))
        } else {
            Some(OllamaFormat::None("json".to_string()))
        };

        let request = OllamaChatRequest {
            model: config.model.clone(),
            messages,
            stream: false,
            options: OllamaChatOptions {
                num_predict: max_tokens,
                temperature: config.temperature,
                top_k: config.top_k,
                top_p: config.top_p,
            },
            format,
        };

        let url = format!("{}/api/chat", config.url);
        debug!(
            "OllamaBackend: sending chat request to {} (model: {}, {} messages)",
            url,
            config.model,
            request.messages.len()
        );

        let response = self.client.post(&url).json(&request).send().await.map_err(|error| {
            if error.is_timeout() {
                LlmError::Timeout(error.to_string())
            } else {
                LlmError::Http(error.to_string())
            }
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LlmError::Http(format!("Ollama /api/chat returned {status}: {body}")));
        }

        let chat_response: OllamaChatResponse = response.json().await.map_err(|error| LlmError::Json(error.to_string()))?;

        let content = chat_response.message.content;
        debug!("OllamaBackend: generated {} chars", content.len());

        Ok((content, conversation))
    }

    async fn clear_conversation(&self) -> Result<(), LlmError> {
        // Ollama is stateless — no KV cache to clear.
        debug!("OllamaBackend: clear_conversation (no-op, Ollama is stateless)");
        Ok(())
    }

    async fn trim_context(&self, _keep_last_n: usize) -> Result<(), LlmError> {
        // Ollama is stateless — no KV cache to trim.
        debug!("OllamaBackend: trim_context (no-op, Ollama is stateless)");
        Ok(())
    }

    async fn reload_model(&self, config: LlmBackendConfig) -> Result<(), LlmError> {
        match config {
            LlmBackendConfig::Ollama(ollama_config) => {
                debug!("OllamaBackend: switching to model {} at {}", ollama_config.model, ollama_config.url);
                let mut current = self.config.write().map_err(|_| LlmError::ChannelClosed)?;
                *current = ollama_config;
                Ok(())
            }
            LlmBackendConfig::Local(_) => Err(LlmError::BackendInit("Cannot reload Local config on OllamaBackend".to_string())),
        }
    }

    async fn update_context_config(&self, _context_config: ContextConfig) -> Result<(), LlmError> {
        // Ollama is stateless — context config is not applicable.
        // The rolling window trimming is handled by the ReAct loop itself.
        debug!("OllamaBackend: update_context_config (no-op, Ollama is stateless)");
        Ok(())
    }

    fn max_tokens(&self) -> usize {
        self.config().max_tokens
    }

    fn set_max_tokens(&self, max_tokens: usize) {
        if let Ok(mut config) = self.config.write() {
            config.max_tokens = max_tokens;
        }
    }

    fn backend_type(&self) -> LlmBackendType {
        LlmBackendType::Ollama
    }

    fn resource_report(&self, last_tool_calls: Vec<String>) -> LlmResourceReport {
        let config = self.config();
        LlmResourceReport {
            backend_type: "ollama".to_string(),
            model_path: None,
            n_ctx: None,
            n_batch: None,
            max_tokens: Some(config.max_tokens),
            temperature: Some(config.temperature),
            top_k: Some(config.top_k),
            top_p: Some(config.top_p),
            n_threads: None,
            context_overflow_threshold: None,
            n_gpu_layers: None,
            rolling_window_keep_last: None,
            context_keep_ratio: None,
            min_preserve_tokens: None,
            ollama_url: Some(config.url),
            ollama_model: Some(config.model),
            last_tool_calls,
        }
    }
}
