# Concept: Voice Assistant Ollama Backend

This document describes the concept for adding an **Ollama backend** to the Voice Assistant service, alongside the existing **local llama.cpp backend**. The
goal is to give users the choice between running the LLM locally (current implementation) or connecting to an externally running Ollama instance (e.g. with
OpenWebUI).

---

## 1. Motivation

The Voice Assistant currently uses `llama-cpp-4` to run GGUF models in-process. This works well for edge deployments with modest hardware, but has limitations:

- **GPU contention**: The in-process LLM competes with Whisper, VAD, and embedding models for VRAM.
- **Model size**: Large models (e.g. 70B) cannot fit in consumer VRAM but may run on a dedicated GPU server.
- **Model management**: Ollama provides `ollama pull`, version pinning, and a model registry — no manual GGUF downloads or `fallback_models.toml` needed.
- **Multi-user**: An Ollama instance can serve multiple clients (e.g. launcher + other tools).
- **OpenWebUI integration**: Users already running Ollama with OpenWebUI can reuse the same model and context.

Adding an Ollama backend lets users pick the deployment model that fits their hardware and workflow.

---

## 2. Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    VoiceAssistantService                 │
│                                                         │
│   ┌─────────────┐        ┌─────────────────────────┐    │
│   │  STT (Whisper)│       │   ReAct Loop (react.rs) │    │
│   └─────────────┘        │                         │    │
│                          │  worker.generate(...)    │    │
│   ┌─────────────┐        │  worker.clear_conversation│   │
│   │  TTS (Piper) │       │  worker.trim_context(...) │   │
│   └─────────────┘        │  worker.reload_model(...) │   │
│                          └────────┬────────────────┘    │
│                                   │                     │
│                          ┌────────▼────────────────┐    │
│                          │   LlmBackend (trait)     │    │
│                          │                         │    │
│                          │  ┌──────────────────┐   │    │
│                          │  │ LocalLlmBackend   │   │    │
│                          │  │ (LlmWorker)       │   │    │
│                          │  └──────────────────┘   │    │
│                          │  ┌──────────────────┐   │    │
│                          │  │ OllamaBackend     │   │    │
│                          │  │ (HTTP client)     │   │    │
│                          │  └──────────────────┘   │    │
│                          └─────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

---

## 3. Crate Structure

No new crates are needed. All changes are within the existing `services/voice_assistant` crate. The `model/voice_assistant` crate gets new config types and MCP
request structs.

| Location                                                | Change                                                        |
|---------------------------------------------------------|---------------------------------------------------------------|
| `services/voice_assistant/src/llm.rs`                   | Add `LlmBackend` trait, wrap `LlmWorker` as `LocalLlmBackend` |
| `services/voice_assistant/src/ollama.rs`                | New file: `OllamaBackend` implementation                      |
| `services/voice_assistant/src/config.rs`                | Add `LlmBackendType` enum and `OllamaConfig` struct           |
| `services/voice_assistant/src/service.rs`               | Backend selection at startup                                  |
| `services/voice_assistant/src/react.rs`                 | Replace `LlmWorker` references with `Arc<dyn LlmBackend>`     |
| `services/voice_assistant/src/mcp/handler/tools.rs`     | Extend `switch_model` for backend switching                   |
| `services/voice_assistant/src/mcp/handler/resources.rs` | Adapt `voice_assistant://llm` resource for both backends      |
| `services/voice_assistant/Cargo.toml`                   | Add `reqwest` and `async-trait` dependencies                  |
| `model/voice_assistant/src/mcp/requests.rs`             | Extend `VoiceAssistantSwitchModelArgs` with `backend` field   |

---

## 4. ChatMessage & LlmBackend Trait

### 4.1 ChatMessage

A backend-agnostic chat message type that eliminates the `llama-cpp-4` dependency from the trait interface. Both backends convert to/from their native
representations internally.

```rust
use serde::Deserialize;
use serde::Serialize;

/// Backend-agnostic chat message.
/// Used throughout the ReAct loop and service layer.
/// `LocalLlmBackend` converts to `LlamaChatMessage` internally;
/// `OllamaBackend` serializes directly to JSON.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message role: "system", "user", or "assistant".
    pub role: String,
    /// Message content text.
    pub content: String,
}

impl ChatMessage {
    pub fn new(role: &str, content: &str) -> Self {
        Self {
            role: role.to_string(),
            content: content.to_string(),
        }
    }

    pub fn user(content: &str) -> Self {
        Self::new("user", content)
    }

    pub fn assistant(content: &str) -> Self {
        Self::new("assistant", content)
    }

    pub fn system(content: &str) -> Self {
        Self::new("system", content)
    }
}
```

### 4.2 LlmBackend Trait

A new trait abstracts the LLM interface so the ReAct loop and service layer are backend-agnostic. The trait uses `ChatMessage` exclusively — no `llama-cpp-4`
types appear in the interface.

```rust
use crate::config::ContextConfig;
use crate::config::LlmConfig;

/// Errors that can occur during LLM inference.
/// Re-exported from llm.rs so both backends use the same error type.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("Failed to initialize backend: {0}")]
    BackendInit(String),
    #[error("Failed to load model: {0}")]
    ModelLoad(String),
    #[error("Failed to create chat message: {0}")]
    ChatMessage(String),
    #[error("HTTP request failed: {0}")]
    HttpError(String),
    #[error("Ollama API error: {0}")]
    OllamaApi(String),
    #[error("Model '{0}' not found in Ollama. Pull it first with: ollama pull {0}")]
    ModelNotFound(String),
    #[error("Request timed out after {0}s")]
    Timeout(u64),
    #[error("Max tokens ({0}) reached")]
    MaxTokensReached(usize),
    #[error("Worker channel closed")]
    ChannelClosed,
}

/// Trait abstracting the LLM inference backend.
/// Implemented by `LocalLlmBackend` (llama.cpp) and `OllamaBackend` (HTTP).
/// Uses `ChatMessage` exclusively — no `llama-cpp-4` types in the interface.
#[async_trait::async_trait]
pub trait LlmBackend: Send + Sync {
    /// Generate a completion from the system prompt and conversation.
    /// Returns (output_text, possibly_trimmed_conversation).
    async fn generate(
        &self,
        system_prompt: &str,
        conversation: Vec<ChatMessage>,
        max_tokens: usize,
        use_grammar: bool,
    ) -> Result<(String, Vec<ChatMessage>), LlmError>;

    /// Clear conversation history (KV cache or server-side context).
    async fn clear_conversation(&self) -> Result<(), LlmError>;

    /// Trim context to keep only the last `keep_last_n` tokens.
    async fn trim_context(&self, keep_last_n: usize) -> Result<(), LlmError>;

    /// Reload the model (local: load GGUF, Ollama: switch model name).
    async fn reload_model(&self, config: LlmConfig) -> Result<(), LlmError>;

    /// Returns the current backend configuration.
    fn config(&self) -> LlmConfig;

    /// Updates max_tokens at runtime.
    fn set_max_tokens(&self, max_tokens: usize);

    /// Updates context configuration at runtime.
    async fn update_context_config(&self, context_config: ContextConfig) -> Result<(), LlmError>;

    /// Returns the backend type for resource reporting.
    fn backend_type(&self) -> LlmBackendType;
}
```

---

## 5. LocalLlmBackend (Wrapper for Existing LlmWorker)

The existing `LlmWorker` already implements all required methods. A thin wrapper delegates to it:

```rust
pub struct LocalLlmBackend {
    worker: LlmWorker,
}

impl LocalLlmBackend {
    pub fn new(engine: LlmInferenceEngine) -> Self {
        Self {
            worker: LlmWorker::spawn(engine),
        }
    }

    /// Converts backend-agnostic `ChatMessage` to `llama-cpp-4`'s `LlamaChatMessage`.
    /// This is the only place where `llama-cpp-4` types appear in the backend layer.
    fn to_llama_messages(messages: &[ChatMessage]) -> Result<Vec<LlamaChatMessage>, LlmError> {
        messages.iter().map(|m| {
            LlamaChatMessage::new(m.role.clone(), m.content.clone())
                .map_err(|e| LlmError::ChatMessage(e.to_string()))
        }).collect()
    }

    /// Converts `LlamaChatMessage` back to `ChatMessage`.
    fn from_llama_messages(messages: &[LlamaChatMessage]) -> Vec<ChatMessage> {
        messages.iter().map(|m| {
            ChatMessage::new(m.role(), m.content())
        }).collect()
    }
}

#[async_trait::async_trait]
impl LlmBackend for LocalLlmBackend {
    async fn generate(&self, system_prompt: &str, conversation: Vec<ChatMessage>, max_tokens: usize, use_grammar: bool) -> Result<(String, Vec<ChatMessage>), LlmError> {
        let llama_messages = Self::to_llama_messages(&conversation)?;
        let (output, trimmed) = self.worker.generate(system_prompt, llama_messages, max_tokens, use_grammar).await?;
        let trimmed_chat = Self::from_llama_messages(&trimmed);
        Ok((output, trimmed_chat))
    }

    async fn clear_conversation(&self) -> Result<(), LlmError> {
        self.worker.clear_conversation().await
    }

    async fn trim_context(&self, keep_last_n: usize) -> Result<(), LlmError> {
        self.worker.trim_context(keep_last_n).await
    }

    async fn reload_model(&self, config: LlmConfig) -> Result<(), LlmError> {
        self.worker.reload_model(config).await
    }

    fn config(&self) -> LlmConfig {
        self.worker.config()
    }

    fn set_max_tokens(&self, max_tokens: usize) {
        self.worker.set_max_tokens(max_tokens);
    }

    async fn update_context_config(&self, context_config: ContextConfig) -> Result<(), LlmError> {
        self.worker.update_context_config(context_config).await
    }

    fn backend_type(&self) -> LlmBackendType {
        LlmBackendType::Local
    }
}
```

**No changes to `LlmWorker` or `LlmInferenceEngine` are needed.** The wrapper handles `ChatMessage` ↔ `LlamaChatMessage` conversion at the boundary. The
`llama-cpp-4` dependency is confined to `LocalLlmBackend` and never leaks into `react.rs` or the trait interface.

---

## 6. OllamaBackend

### 6.1 Ollama API Mapping

| Trait Method              | Ollama Endpoint                 | Notes                                               |
|---------------------------|---------------------------------|-----------------------------------------------------|
| `generate()`              | `POST /api/chat`                | `ChatMessage` serialized directly to JSON           |
| `clear_conversation()`    | Resets `keep_last_n` to default | Ollama is stateless; client resets trimming state   |
| `trim_context()`          | Stores `keep_last_n` locally    | Applied in next `generate()` to trim `conversation` |
| `reload_model()`          | No HTTP call                    | Just updates the model name in config               |
| `set_max_tokens()`        | Updates `predict` field         | Applied on next `generate()` call                   |
| `update_context_config()` | Updates `keep_last_n`           | `rolling_window_keep_last` used for trimming        |

### 6.2 Implementation Sketch

```rust
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::sync::RwLock;

/// Ollama chat message — matches Ollama's `/api/chat` message format.
/// Serialized directly via serde; no manual JSON construction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OllamaChatMessage {
    /// Message role: "system", "user", or "assistant".
    pub role: String,
    /// Message content text.
    pub content: String,
}

impl From<&ChatMessage> for OllamaChatMessage {
    fn from(msg: &ChatMessage) -> Self {
        Self {
            role: msg.role.clone(),
            content: msg.content.clone(),
        }
    }
}

/// Ollama `/api/chat` request options.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OllamaChatOptions {
    /// Maximum number of tokens to predict (maps to Ollama's `num_predict`).
    pub num_predict: usize,
    /// Sampling temperature.
    pub temperature: f32,
    /// Top-k sampling parameter.
    pub top_k: i32,
    /// Top-p sampling parameter.
    pub top_p: f32,
}

/// Format field for the `/api/chat` request.
/// Ollama accepts either a bare string ("json") or a full JSON Schema object.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OllamaFormat {
    /// No format constraint — free-form output.
    None(String),
    /// JSON Schema for Structured Outputs (Ollama 0.5.0+).
    Schema(serde_json::Value),
}

/// Ollama `/api/chat` request body.
/// Serialized via serde; no manual `serde_json::json!()` construction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OllamaChatRequest {
    /// Ollama model tag (e.g. "gemma2:9b-instruct-q4_K_M").
    pub model: String,
    /// Conversation messages including the system prompt.
    pub messages: Vec<OllamaChatMessage>,
    /// Whether to stream the response (always false for ReAct).
    pub stream: bool,
    /// Sampling and prediction options.
    pub options: OllamaChatOptions,
    /// Output format constraint: none, or JSON Schema for structured outputs.
    pub format: OllamaFormat,
}

/// Ollama `/api/chat` response body.
#[derive(Clone, Debug, Deserialize)]
pub struct OllamaChatResponse {
    /// The generated message.
    pub message: OllamaChatResponseMessage,
}

/// The `message` field within the chat response.
#[derive(Clone, Debug, Deserialize)]
pub struct OllamaChatResponseMessage {
    /// Generated content text.
    pub content: String,
}

/// Ollama `/api/tags` response body.
#[derive(Clone, Debug, Deserialize)]
pub struct OllamaTagsResponse {
    /// List of locally available models.
    pub models: Vec<OllamaTagEntry>,
}

/// A single model entry in the `/api/tags` response.
#[derive(Clone, Debug, Deserialize)]
pub struct OllamaTagEntry {
    /// Model tag (e.g. "gemma2:9b-instruct-q4_K_M").
    pub name: String,
}

pub struct OllamaBackend {
    client: Client,
    url: String,
    model: Arc<RwLock<String>>,
    config: Arc<RwLock<LlmConfig>>,
    /// Number of trailing messages to keep when trimming.
    /// Updated by `trim_context()` and `update_context_config()`.
    /// Applied in `generate()` before sending the request to Ollama.
    keep_last_n: Arc<RwLock<usize>>,
}

impl OllamaBackend {
    pub fn new(url: &str, model: &str, config: LlmConfig, connect_timeout_secs: u64, request_timeout_secs: u64) -> Result<Self, LlmError> {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(connect_timeout_secs))
            .timeout(std::time::Duration::from_secs(request_timeout_secs))
            .build()
            .map_err(|e| LlmError::BackendInit(e.to_string()))?;

        let initial_keep_last = config.context_config.rolling_window_keep_last;

        let backend = Self {
            client,
            url: url.to_string(),
            model: Arc::new(RwLock::new(model.to_string())),
            config: Arc::new(RwLock::new(config)),
            keep_last_n: Arc::new(RwLock::new(initial_keep_last)),
        };

        // Validate that the model is available in the Ollama instance.
        // This catches typos and missing `ollama pull` early, before the
        // first chat request fails with a cryptic HTTP error.
        // Note: This is a blocking call. `new()` is called from a sync context
        // in service.rs, so we use `tokio::runtime::Handle` or a blocking
        // runtime internally. Alternatively, make `new()` async and call
        // it from the service's async init.
        //
        // For the concept, we show the sync variant. In the implementation,
        // `new()` can be made async or wrapped in `tokio::task::block_in_place`.
        // A simpler approach: make `check_model_available` a separate method
        // called from the async service init after `new()`.
        //
        // The recommended approach is to split construction and validation:
        //   1. `OllamaBackend::new()` — creates the struct (no I/O)
        //   2. `backend.check_model_available()` — async validation
        // This keeps `new()` cheap and lets the service handle validation
        // errors with proper logging and fallback logic.

        Ok(backend)
    }

    /// Checks whether the configured model is available in the Ollama instance.
    /// Calls `GET /api/tags` and compares against the model name.
    /// Returns an error with a helpful `ollama pull` hint if the model is missing.
    ///
    /// Should be called after `new()` during service initialization and
    /// after `reload_model()` before switching to the new model.
    pub async fn check_model_available(&self) -> Result<(), LlmError> {
        let model = self.model.read().map(|m| m.clone()).map_err(|_| LlmError::ChannelClosed)?;

        let response = self.client
            .get(format!("{}/api/tags", self.url))
            .send()
            .await
            .map_err(|e| LlmError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(LlmError::OllamaApi(format!("GET /api/tags returned HTTP {status}: {text}")));
        }

        let tags: OllamaTagsResponse = response.json().await
            .map_err(|e| LlmError::HttpError(e.to_string()))?;

        // Ollama model tags may include or omit the tag suffix (e.g. "gemma2:9b" vs "gemma2:latest").
        // Match on the base name (before ':') to be lenient.
        let model_base = model.split(':').next().unwrap_or(&model);
        let found = tags.models.iter().any(|entry| {
            let name_base = entry.name.split(':').next().unwrap_or(&entry.name);
            name_base == model_base
        });

        if !found {
            return Err(LlmError::ModelNotFound(model));
        }

        Ok(())
    }

    /// Trims the conversation to the last `keep_last_n` messages.
    /// Preserves the system prompt (first message) if present.
    /// Called internally by `generate()` before sending to Ollama.
    fn trim_conversation(messages: &mut Vec<ChatMessage>, keep_last_n: usize) {
        if messages.len() <= keep_last_n {
            return;
        }
        // Keep the last `keep_last_n` messages.
        // The system prompt is always prepended by `generate()` separately,
        // so we don't need to preserve it from the conversation Vec.
        let split_point = messages.len() - keep_last_n;
        messages.drain(0..split_point);
    }
}

#[async_trait::async_trait]
impl LlmBackend for OllamaBackend {
    async fn generate(
        &self,
        system_prompt: &str,
        mut conversation: Vec<ChatMessage>,
        max_tokens: usize,
        use_grammar: bool,
    ) -> Result<(String, Vec<ChatMessage>), LlmError> {
        let model = self.model.read().map_err(|_| LlmError::ChannelClosed)?.clone();

        // Trim conversation to keep_last_n messages before sending.
        // This prevents exceeding Ollama's num_ctx token limit in long ReAct loops.
        let keep_last_n = self.keep_last_n.read().map(|n| *n).unwrap_or(6);
        Self::trim_conversation(&mut conversation, keep_last_n);

        // Build the request using typed structs — no manual serde_json::json!().
        let mut messages = Vec::with_capacity(conversation.len() + 1);
        messages.push(OllamaChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        });
        for msg in &conversation {
            messages.push(OllamaChatMessage::from(msg));
        }

        let config = self.config.read().map(|c| c.clone()).unwrap_or_default();

        // When use_grammar is true, pass a full JSON Schema (Ollama Structured Outputs)
        // instead of just "json". This constrains the output to the exact ReAct format,
        // preventing parse errors that a bare `format: "json"` would allow.
        //
        // The schema enforces one of five action types via `oneOf`:
        //   {"tool": "<name>", "parameters": {}}
        //   {"resource": "<uri>"}
        //   {"final_answer": "<text>", "new_insights": [...]}
        //   {"clarify": {"question": "..."}}
        //   {"text_to_speech_answer": "<text>"}
        //
        // Ollama's Structured Outputs feature uses llama.cpp's grammar sampler
        // internally, so this is equivalent to the local backend's GBNF grammar
        // but with schema-level validation instead of a raw GBNF string.
        let format = if use_grammar {
            OllamaFormat::Schema(REACT_JSON_SCHEMA.clone())
        } else {
            OllamaFormat::None(String::new())
        };

        let request = OllamaChatRequest {
            model: model.clone(),
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

        let response = self.client
            .post(format!("{}/api/chat", self.url))
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(LlmError::OllamaApi(format!("HTTP {status}: {text}")));
        }

        let chat_response: OllamaChatResponse = response.json().await
            .map_err(|e| LlmError::HttpError(e.to_string()))?;

        let content = chat_response.message.content;

        Ok((content, conversation))
    }

    async fn clear_conversation(&self) -> Result<(), LlmError> {
        // Reset trimming state to default. Ollama is stateless per request,
        // so there's no server-side context to clear — only our local
        // keep_last_n tracking needs resetting.
        let default_keep = self.config.read()
            .map(|c| c.context_config.rolling_window_keep_last)
            .unwrap_or(6);
        if let Ok(mut keep) = self.keep_last_n.write() {
            *keep = default_keep;
        }
        Ok(())
    }

    async fn trim_context(&self, keep_last_n: usize) -> Result<(), LlmError> {
        // Store the trimming threshold. Applied on the next `generate()` call
        // to trim the conversation Vec before sending to Ollama.
        // This mirrors the local backend's KV cache shift: old messages are
        // dropped to make room for new ones within Ollama's num_ctx limit.
        if let Ok(mut keep) = self.keep_last_n.write() {
            *keep = keep_last_n;
        }
        Ok(())
    }

    async fn reload_model(&self, config: LlmConfig) -> Result<(), LlmError> {
        // For Ollama, "reloading" means switching the model name.
        // The model must already be pulled via `ollama pull <model>`.
        let new_model = config.model_path.clone();

        // Validate the new model is available before switching.
        // If the model is missing, return an error without updating the
        // current model — the caller can fall back to the previous model.
        let old_model = self.model.read().map(|m| m.clone()).unwrap_or_default();
        if let Ok(mut model) = self.model.write() {
            *model = new_model.clone();
        }
        match self.check_model_available().await {
            Ok(()) => {
                if let Ok(mut cfg) = self.config.write() {
                    *cfg = config;
                }
                Ok(())
            }
            Err(error) => {
                // Revert to the old model on validation failure.
                if let Ok(mut model) = self.model.write() {
                    *model = old_model;
                }
                Err(error)
            }
        }
    }

    fn config(&self) -> LlmConfig {
        self.config.read().map(|c| c.clone()).unwrap_or_default()
    }

    fn set_max_tokens(&self, max_tokens: usize) {
        if let Ok(mut config) = self.config.write() {
            config.max_tokens = max_tokens;
        }
    }

    async fn update_context_config(&self, context_config: ContextConfig) -> Result<(), LlmError> {
        // Update keep_last_n from the new context config.
        // rolling_window_keep_last controls how many trailing messages
        // are preserved when trimming in generate().
        if let Ok(mut keep) = self.keep_last_n.write() {
            *keep = context_config.rolling_window_keep_last;
        }
        if let Ok(mut config) = self.config.write() {
            config.context_config = context_config;
        }
        Ok(())
    }

    fn backend_type(&self) -> LlmBackendType {
        LlmBackendType::Ollama
    }
}
```

### 6.3 ReAct JSON Schema for Structured Outputs

Ollama's Structured Outputs feature accepts a full JSON Schema in the `format` field. This is strictly more powerful than passing `"json"` (which only
guarantees valid JSON but not the expected shape). The schema enforces the ReAct action format so the Rust parser in `react.rs` receives exactly the keys it
expects.

The schema is loaded from a `.json` file via `include_str!` and parsed once at startup. This keeps the schema definition in a dedicated file that can be
validated independently and edited without touching Rust code.

**File**: `services/voice_assistant/data/react_schema.json`

```json
{
  "type": "object",
  "oneOf": [
    {
      "type": "object",
      "properties": {
        "tool": {
          "type": "string"
        },
        "parameters": {
          "type": "object"
        }
      },
      "required": [
        "tool"
      ],
      "additionalProperties": false
    },
    {
      "type": "object",
      "properties": {
        "resource": {
          "type": "string"
        }
      },
      "required": [
        "resource"
      ],
      "additionalProperties": false
    },
    {
      "type": "object",
      "properties": {
        "final_answer": {
          "type": "string"
        },
        "new_insights": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "category": {
                "type": "string"
              },
              "fact": {
                "type": "string"
              }
            },
            "required": [
              "category",
              "fact"
            ],
            "additionalProperties": false
          }
        }
      },
      "required": [
        "final_answer"
      ],
      "additionalProperties": false
    },
    {
      "type": "object",
      "properties": {
        "clarify": {
          "type": "object",
          "properties": {
            "question": {
              "type": "string"
            }
          },
          "required": [
            "question"
          ],
          "additionalProperties": false
        }
      },
      "required": [
        "clarify"
      ],
      "additionalProperties": false
    },
    {
      "type": "object",
      "properties": {
        "text_to_speech_answer": {
          "type": "string"
        }
      },
      "required": [
        "text_to_speech_answer"
      ],
      "additionalProperties": false
    }
  ]
}
```

```rust
/// JSON Schema for ReAct structured output.
/// Passed to Ollama's `format` field when `use_grammar` is true.
/// Uses `oneOf` to enforce exactly one action type per response.
///
/// Loaded from `data/react_schema.json` via `include_str!` and parsed once
/// at startup. The `.json` file can be validated independently and edited
/// without recompiling Rust code (though `include_str!` still embeds it at
/// compile time — use `std::fs::read_to_string` instead if runtime loading
/// is desired).
const REACT_JSON_SCHEMA: once_cell::sync::Lazy<serde_json::Value> = once_cell::sync::Lazy::new(|| {
    serde_json::from_str(include_str!("../data/react_schema.json"))
        .expect("react_schema.json must be valid JSON")
});
```

**Compatibility note**: Ollama's Structured Outputs requires Ollama 0.5.0+ (which uses llama.cpp's grammar sampler internally). If the connected Ollama instance
is older, the `format` field with a schema object is ignored and the model produces free-form JSON. The Rust parser in `react.rs` already handles malformed
output via `sanitize_llm_output()` and `repair_malformed_json()`, so the system degrades gracefully.

### 6.4 No LlamaChatMessage Dependency in Interface

The `ChatMessage` type (defined in section 4.1) is used throughout the trait interface, ReAct loop, and service layer. `llama-cpp-4`'s `LlamaChatMessage` is
confined to `LocalLlmBackend`'s internal conversion logic (section 5). `OllamaBackend` serializes `ChatMessage` directly to JSON — no `llama-cpp-4` types
needed.

This means `react.rs` no longer imports `llama_cpp_4::model::LlamaChatMessage`, making the ReAct loop fully backend-agnostic.

---

## 7. Configuration

### 7.1 Config Extension in `config.rs`

```rust
/// LLM backend type selection.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub enum LlmBackendType {
    /// Local llama.cpp inference (GGUF models, in-process).
    #[default]
    Local,
    /// Remote Ollama server (HTTP API).
    Ollama,
}

impl FromStr for LlmBackendType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" | "llama" | "llama_cpp" => Ok(LlmBackendType::Local),
            "ollama" | "openwebui" => Ok(LlmBackendType::Ollama),
            _ => Err(format!("unknown LLM backend '{s}', expected: local, ollama")),
        }
    }
}

/// Configuration for the Ollama backend.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct OllamaConfig {
    /// Ollama server URL (e.g. "http://localhost:11434").
    pub url: String,
    /// Ollama model tag (e.g. "gemma2:9b-instruct-q4_K_M").
    pub model: String,
    /// Connect timeout in seconds — time to establish the TCP connection.
    /// Short default to fail fast if the Ollama server is unreachable.
    pub connect_timeout_seconds: u64,
    /// Request timeout in seconds — total time for a single chat request
    /// including model inference. 30s is a reasonable default for interactive
    /// ReAct loops; increase for large models on slow hardware.
    pub request_timeout_seconds: u64,
    /// Optional API key for authenticated Ollama proxies.
    pub api_key: String,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:11434".to_string(),
            model: String::new(),
            connect_timeout_seconds: 5,
            request_timeout_seconds: 30,
            api_key: String::new(),
        }
    }
}
```

### 7.2 Service Config Extension

Add two fields to `VoiceAssistantServiceConfig`:

```rust
pub struct VoiceAssistantServiceConfig {
    // ... existing fields ...

    /// LLM backend type: "local" (llama.cpp) or "ollama" (HTTP).
    #[serde(default)]
    pub llm_backend: LlmBackendType,

    /// Ollama backend configuration (used when llm_backend == "ollama").
    #[serde(default)]
    pub ollama: OllamaConfig,
}
```

### 7.3 TOML Configuration Example

```toml
[voice_assistant]
# Option A: Local llama.cpp (default)
llm_backend = "local"
llm_model_path = "~/.local/share/smearor/models/gemma-4-E4B-it-qat-UD-Q4_K_XL.gguf"
llm_threads = 8
llm_context_size = 8192

# Option B: Ollama
# llm_backend = "ollama"
# [voice_assistant.ollama]
# url = "http://localhost:11434"
# model = "gemma2:9b-instruct-q4_K_M"
# connect_timeout_seconds = 5
# request_timeout_seconds = 30
# api_key = ""
```

When `llm_backend = "ollama"`, the `llm_model_path`, `llm_threads`, `n_gpu_layers`, `use_grammar`, and GPU-related fields are ignored. The `llm_context_size`,
`llm_max_tokens`, and `llm_temperature` are still used (passed as Ollama `options`).

---

## 8. Service Initialization

In `service.rs`, the startup logic changes from:

```rust
// Current:
let llm_config = service.config.to_llm_config();
match LlmInferenceEngine::load( & llm_config) {
Ok(engine) => {
let worker = LlmWorker::spawn(engine);
service.llm_worker = Some(Arc::new(worker));
}
Err(error) => {
error ! ("Voice Assistant: Failed to load LLM engine: {error}");
}
}
```

To:

```rust
// New:
let backend: Arc<dyn LlmBackend> = match service.config.llm_backend {
LlmBackendType::Local => {
let llm_config = service.config.to_llm_config();
match LlmInferenceEngine::load( & llm_config) {
Ok(engine) => Arc::new(LocalLlmBackend::new(engine)),
Err(error) => {
error ! ("Voice Assistant: Failed to load local LLM engine: {error}");
return; // or fall back to a no-op backend
}
}
}
LlmBackendType::Ollama => {
let ollama_cfg = & service.config.ollama;
let llm_config = service.config.to_llm_config();
// For Ollama, model_path is the Ollama model tag
let mut ollama_llm_config = llm_config;
ollama_llm_config.model_path = ollama_cfg.model.clone();
match OllamaBackend::new( & ollama_cfg.url, &ollama_cfg.model, ollama_llm_config, ollama_cfg.connect_timeout_seconds, ollama_cfg.request_timeout_seconds) {
Ok(backend) => {
// Validate that the model is available in the Ollama instance.
// This catches typos and missing `ollama pull` before the first
// chat request fails with a cryptic error.
match backend.check_model_available().await {
Ok(()) => Arc::new(backend),
Err(error) => {
error ! ("Voice Assistant: Ollama model '{}' not available: {error}", ollama_cfg.model);
error ! ("Voice Assistant: Run 'ollama pull {}' to download the model", ollama_cfg.model);
return;
}
}
}
Err(error) => {
error ! ("Voice Assistant: Failed to initialize Ollama backend: {error}");
return;
}
}
}
};
service.llm_backend = Some(backend);
```

### 8.1 Service Struct Change

Replace:

```rust
pub llm_engine: Option<Arc<LlmInferenceEngine> >,
pub llm_worker: Option<Arc<LlmWorker> >,
```

With:

```rust
use arc_swap::ArcSwapOption;

pub struct VoiceAssistantService {
    // ... other fields ...

    /// LLM backend, swappable at runtime via MCP tools.
    /// Uses `ArcSwapOption` for lock-free, thread-safe swaps.
    /// Readers (ReAct loop, resource handler) call `.load()` to get an
    /// `Guard<Arc<dyn LlmBackend>>` — no locking, no contention.
    /// Writers (switch_model, switch_backend) call `.store()` to atomically
    /// replace the backend.
    pub llm_backend: ArcSwapOption<dyn LlmBackend>,
}
```

**Why `ArcSwapOption` instead of `Option<Arc<...>>` or `RwLock<Option<...>>`:**

| Approach                           | Read Path             | Write Path             | Suitability                                     |
|------------------------------------|-----------------------|------------------------|-------------------------------------------------|
| `Option<Arc<dyn LlmBackend>>`      | `&self` access only   | Requires `&mut self`   | Not thread-safe for runtime swaps               |
| `RwLock<Option<Arc<...>>>`         | `.read()` lock        | `.write()` lock        | Works, but adds lock overhead on every read     |
| `tokio::sync::RwLock<Option<...>>` | `.read().await`       | `.write().await`       | Async-safe, but adds await overhead on hot path |
| `ArcSwapOption<dyn LlmBackend>`    | `.load()` (lock-free) | `.store()` (lock-free) | Best: zero contention on reads, atomic swaps    |

The ReAct loop calls `self.llm_backend.load()` on every iteration — `ArcSwapOption` makes this a single atomic load with no locking. The MCP `switch_model` /
`switch_backend` handler calls `.store()` to atomically replace the backend, which is safe even while a `generate()` call is in progress (the old `Arc` stays
alive until all readers drop it).

All references in `react.rs`, `tools.rs`, and `resources.rs` are updated from `self.llm_worker.as_ref()` to `self.llm_backend.load()`. The `llm_engine` field
can be removed or kept as `Option<Arc<LlmInferenceEngine>>` for the local backend (needed for `LlmInferenceEngine::model()` access in resource reporting).

### 8.2 Access Patterns

```rust
// Read (ReAct loop, resource handler, tool handler):
let backend = self .llm_backend.load();
if let Some(backend) = backend.as_ref() {
backend.generate(...).await ?;
}
// Guard is dropped automatically when `backend` goes out of scope.

// Write (switch_model / switch_backend MCP tool):
let new_backend: Arc<dyn LlmBackend> = match new_backend_type {
LlmBackendType::Local => Arc::new(LocalLlmBackend::new(engine)),
LlmBackendType::Ollama => Arc::new(OllamaBackend::new(...) ? ),
};
self .llm_backend.store(Some(new_backend));
// Old backend's Arc is decremented; if no readers hold it, it's dropped.
// If a generate() is in-flight, the old Arc stays alive until it completes.
```

---

## 9. ReAct Loop Changes (`react.rs`)

The ReAct loop currently accesses `self.llm_worker` directly. After refactoring, it uses `self.llm_backend.load()` to obtain a `Guard<Arc<dyn LlmBackend>>`. The
method signatures are identical, so the only change is the field access pattern.

### 9.1 ChatMessage Usage

The ReAct loop uses `ChatMessage` (defined in section 4.1) instead of `LlamaChatMessage`. The change from:

```rust
active_payload.push(LlamaChatMessage::new("user".to_string(), context_message)...);
```

To:

```rust
active_payload.push(ChatMessage::user( & context_message));
```

The `react.rs` module no longer imports `llama_cpp_4::model::LlamaChatMessage`. The `LocalLlmBackend` handles `ChatMessage` ↔ `LlamaChatMessage` conversion
internally (see section 5). `OllamaBackend` serializes `ChatMessage` directly to JSON.

### 9.2 Feature Differences Between Backends

| Feature                         | Local Backend           | Ollama Backend                               |
|---------------------------------|-------------------------|----------------------------------------------|
| KV cache management             | Active (rolling window) | N/A (no client-side KV cache)                |
| GBNF grammar enforcement        | Active (`use_grammar`)  | Replaced by JSON Schema (Structured Outputs) |
| GPU layer offloading            | Active (`n_gpu_layers`) | N/A (Ollama manages)                         |
| Context trimming                | Active (KV cache shift) | Active (client-side Vec trim)                |
| Model download (`ensure_model`) | HuggingFace fallback    | `ollama pull` (manual)                       |

---

## 10. MCP Tool Extension

### 10.1 `voice_assistant_switch_model` Enhancement

Extend `VoiceAssistantSwitchModelArgs` in `model/voice_assistant/src/mcp/requests.rs`:

```rust
pub struct VoiceAssistantSwitchModelArgs {
    /// Path to the new GGUF model file (local backend) or Ollama model tag (ollama backend).
    pub model_path: String,
    /// Override the context window size. Omit to use the configured default.
    pub n_ctx: Option<i32>,
    /// Override the max tokens to generate per response.
    pub max_tokens: Option<i32>,
    /// When true, download the model if it doesn't exist locally.
    pub ensure_model: Option<bool>,
    /// Switch the LLM backend at runtime: "local" or "ollama".
    /// If omitted, keeps the current backend.
    pub backend: Option<String>,
}
```

### 10.2 Handler Logic (`tools.rs`)

```rust
VoiceAssistantMcpTools::SwitchModel => {
let args: VoiceAssistantSwitchModelArgs = serde_json::from_str(...).unwrap_or_default();

// If backend switch is requested, atomically swap the backend.
// ArcSwapOption::store() is lock-free — safe even while a
// generate() call is in-flight on the old backend.
if let Some(ref backend_str) = args.backend {
let new_backend_type = LlmBackendType::from_str(backend_str).unwrap_or(LlmBackendType::Local);
let new_backend: Arc < dyn LlmBackend > = match new_backend_type {
LlmBackendType::Local => {
let llm_config = self.config.to_llm_config_with_model( &path, ...);
let engine = LlmInferenceEngine::load( & llm_config) ?;
Arc::new(LocalLlmBackend::new(engine))
}
LlmBackendType::Ollama => {
let ollama_cfg = & self.config.ollama;
let backend = OllamaBackend::new( & ollama_cfg.url, & ollama_cfg.model, ..., ollama_cfg.connect_timeout_seconds, ollama_cfg.request_timeout_seconds) ?;
backend.check_model_available().await ?;
Arc::new(backend)
}
};
self.llm_backend.store(Some(new_backend));
}

// Then switch the model within the (possibly new) backend.
// .load() returns a Guard; .as_ref() gives Option<&Arc<dyn LlmBackend>>.
let backend_guard = self.llm_backend.load();
match backend_guard.as_ref() {
Some(backend) => {
let config = self.config.to_llm_config_with_model( &path, ...);
backend.reload_model(config).await ?;
}
None => { ... }
}
}
```

### 10.3 New MCP Tool: `voice_assistant_switch_backend`

Alternatively, a dedicated tool can be cleaner:

```rust
pub struct VoiceAssistantSwitchBackendArgs {
    /// Backend type: "local" or "ollama"
    pub backend: String,
    /// Ollama URL (only used when backend = "ollama"). Defaults to config value.
    pub ollama_url: Option<String>,
    /// Ollama model tag (only used when backend = "ollama"). Defaults to config value.
    pub ollama_model: Option<String>,
}
```

**Recommendation**: Start with extending `switch_model` with the `backend` field. Add a dedicated `switch_backend` tool later if the logic gets too complex.

---

## 11. Resource Adaptation (`resources.rs`)

The `voice_assistant://llm` resource currently reports local-only fields. It needs to adapt:

```rust
/// Report returned by the `voice_assistant://llm` resource.
/// Serialized via serde; no manual `serde_json::json!()` construction.
#[derive(Clone, Debug, Serialize)]
pub struct LlmResourceReport {
    /// Active backend type ("local" or "ollama").
    pub backend: String,
    /// Model path (local) or model tag (Ollama).
    pub model_path: String,
    /// Maximum tokens to generate per response.
    pub max_tokens: usize,
    /// Sampling temperature.
    pub temperature: f32,
    /// Context window size.
    pub n_ctx: u32,
    /// Recent tool calls for debugging.
    pub last_tool_calls: Vec<String>,
    /// Local-only: number of CPU threads (null for Ollama).
    pub n_threads: Option<i32>,
    /// Local-only: GPU layer count (null for Ollama).
    pub n_gpu_layers: Option<i32>,
    /// Local-only: batch size (null for Ollama).
    pub n_batch: Option<u32>,
    /// Ollama-only: server URL (null for local).
    pub ollama_url: Option<String>,
}

VoiceAssistantMcpResources::Llm => {
    let backend_guard = self.llm_backend.load();
    let json = if let Some(backend) = backend_guard.as_ref() {
        let cfg = backend.config();
        let backend_type = backend.backend_type();
        let report = LlmResourceReport {
            backend: format!("{:?}", backend_type),
            model_path: cfg.model_path,
            max_tokens: cfg.max_tokens,
            temperature: cfg.temperature,
            n_ctx: cfg.n_ctx,
            last_tool_calls: tool_calls,
            n_threads: if matches!(backend_type, LlmBackendType::Local) { Some(cfg.n_threads) } else { None },
            n_gpu_layers: if matches!(backend_type, LlmBackendType::Local) { Some(cfg.gpu_config.n_gpu_layers) } else { None },
            n_batch: if matches!(backend_type, LlmBackendType::Local) { Some(cfg.n_batch) } else { None },
            ollama_url: if matches!(backend_type, LlmBackendType::Ollama) { Some(self.config.ollama.url.clone()) } else { None },
        };
        serde_json::to_value(&report).unwrap_or(serde_json::Value::Null)
    } else {
        serde_json::json!({"error": "LLM backend not initialized"})
    };
}
```

---

## 12. Dependencies

### 12.1 New Crate Dependencies

Add to `services/voice_assistant/Cargo.toml`:

```toml
[dependencies]
# ... existing ...
arc-swap = "1.7"
async-trait = "0.1"
once_cell = { version = "1.20", features = ["std"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
```

`reqwest` with `rustls-tls` avoids adding OpenSSL as a system dependency. The `json` feature enables `.json()` on requests and responses.

### 12.2 Conditional Compilation

The `llama-cpp-4` dependency is still required for the local backend. To allow building an Ollama-only variant (without llama.cpp), we could feature-gate it:

```toml
[features]
default = ["llm-local"]
llm-local = ["llama-cpp-4"]
llm-ollama = ["dep:reqwest", "dep:async-trait"]
```

This is optional — for now, both backends can be compiled in unconditionally. The feature-gating can be added later if a minimal Ollama-only package is desired.

---

## 13. Debian Packaging

### 13.1 Ollama-Only Variant (Optional)

An Ollama-only `.deb` package would be significantly smaller (no `libllama.so`, `libggml*.so`):

```toml
[package.metadata.deb.variants.ollama]
name = "smearor-service-voice-assistant-ollama"
depends = "libstdc++6, libssl3t64, libglib2.0-0t64, libasound2t64, smearor-swipe-launcher (>= 0.1.0), smearor-service-personalization (>= 0.1.0)"
conflicts = "smearor-service-voice-assistant, smearor-service-voice-assistant-vulkan, smearor-service-voice-assistant-hipblas"
replaces = "smearor-service-voice-assistant, smearor-service-voice-assistant-vulkan, smearor-service-voice-assistant-hipblas"
assets = [
    ["target/release/libsmearor_voice_assistant_service.so", "/usr/lib/smearor/", "755"],
]
```

### 13.2 Build Script Extension

`scripts/build-deb.sh` gets a new variant:

```bash
case "$BUILD_VARIANT" in
    ollama)
        VOICE_ASSISTANT_FEATURES="llm-ollama"
        VARIANT_LABEL="Ollama (remote LLM)"
        ;;
esac
```

---

## 14. Implementation Phases

### Phase 1: Trait Abstraction & LocalLlmBackend Wrapper

- Add `LlmBackend` trait to `llm.rs`
- Add `LocalLlmBackend` wrapper around `LlmWorker`
- Introduce `ChatMessage` struct (backend-agnostic)
- Refactor `react.rs` to use `Arc<dyn LlmBackend>` and `ChatMessage`
- Update `service.rs` to hold `llm_backend: Option<Arc<dyn LlmBackend>>`
- Update `resources.rs` and `tools.rs` references
- **Exit criteria**: All existing functionality works identically. No behavior change.

### Phase 2: OllamaBackend Implementation

- Add `ollama.rs` with `OllamaBackend` struct
- Implement `LlmBackend` trait for `OllamaBackend`
- Add `reqwest` and `async-trait` dependencies
- Unit tests with mock HTTP server (or integration test against local Ollama)
- **Exit criteria**: `OllamaBackend` passes all trait method tests.

### Phase 3: Config & Service Integration

- Add `LlmBackendType`, `OllamaConfig` to `config.rs`
- Add `llm_backend` and `ollama` fields to `VoiceAssistantServiceConfig`
- Update service initialization to select backend based on config
- Update `to_llm_config()` to handle Ollama model path
- **Exit criteria**: Service starts with either backend based on TOML config.

### Phase 4: MCP Tool Enhancement

- Extend `VoiceAssistantSwitchModelArgs` with `backend` field
- Update `switch_model` handler to support runtime backend switching
- Update `voice_assistant://llm` resource for backend-aware reporting
- **Exit criteria**: Runtime backend switching works via MCP tool.

### Phase 5: Debian Packaging (Optional)

- Add `ollama` variant to `Cargo.toml` `[package.metadata.deb.variants]`
- Update `build-deb.sh` with `ollama` build variant
- Feature-gate `llama-cpp-4` behind `llm-local` feature
- **Exit criteria**: `./scripts/build-deb.sh ollama` produces a working `.deb`.

---

## 15. Limitations & Trade-Offs

| Aspect              | Local (llama.cpp)                    | Ollama                                   |
|---------------------|--------------------------------------|------------------------------------------|
| Latency             | ~50-500ms (in-process)               | ~100-2000ms (HTTP + server)              |
| GPU sharing         | Contends with Whisper/VAD            | Ollama manages its own GPU               |
| Model format        | GGUF only                            | GGUF (via Ollama registry)               |
| Grammar enforcement | GBNF (native)                        | JSON Schema via Structured Outputs       |
| Context management  | Fine-grained (KV cache shift)        | Client-side Vec trimming (`keep_last_n`) |
| Offline operation   | Yes                                  | No (needs Ollama server running)         |
| Memory footprint    | Full model in process VRAM           | Minimal (HTTP client only)               |
| Model download      | HuggingFace / `fallback_models.toml` | `ollama pull <model>`                    |
| Streaming           | Not implemented                      | Possible via `stream: true`              |

---

## 16. Future Extensions

- **Streaming responses**: Ollama supports `stream: true` for token-by-token output. This could enable TTS to start speaking before the full response is
  generated (barge-in).
- **OpenAI-compatible API**: The Ollama backend could be generalized to support any OpenAI-compatible API endpoint (e.g. vLLM, llama.cpp server, LM Studio) by
  making the URL and API key configurable.
- **Backend auto-fallback**: If the local backend fails to load (e.g. model not found), automatically fall back to Ollama if configured.
- **Periodic health check**: Extend `check_model_available()` into a periodic `GET /api/tags` poll to verify Ollama server availability and report status via
  `voice_assistant://llm` resource.
