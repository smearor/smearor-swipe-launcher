use async_trait::async_trait;
use tracing::debug;

use crate::config::ContextConfig;
use crate::llm::LlmWorker;

use super::ChatMessage;
use super::LlmBackend;
use super::LlmBackendConfig;
use super::LlmBackendType;
use super::LlmError;
use super::LlmResourceReport;

/// Local LLM backend wrapping the existing `LlmWorker` (llama.cpp via `llama-cpp-4`).
///
/// Converts `ChatMessage` to `LlamaChatMessage` internally, keeping
/// `llama-cpp-4` types confined to this module.
pub struct LocalLlmBackend {
    worker: LlmWorker,
}

impl LocalLlmBackend {
    /// Creates a new local backend wrapping the given worker.
    pub fn new(worker: LlmWorker) -> Self {
        Self { worker }
    }

    /// Converts `ChatMessage` instances to `LlamaChatMessage` instances.
    fn convert_messages(messages: &[ChatMessage]) -> Result<Vec<llama_cpp_4::model::LlamaChatMessage>, LlmError> {
        messages
            .iter()
            .map(|msg| {
                llama_cpp_4::model::LlamaChatMessage::new(msg.role.as_str().to_string(), msg.content.clone())
                    .map_err(|error| LlmError::ChatMessage(error.to_string()))
            })
            .collect()
    }
}

#[async_trait]
impl LlmBackend for LocalLlmBackend {
    async fn generate(
        &self,
        system_prompt: &str,
        conversation: Vec<ChatMessage>,
        max_tokens: usize,
        use_grammar: bool,
    ) -> Result<(String, Vec<ChatMessage>), LlmError> {
        let llama_messages = Self::convert_messages(&conversation)?;
        let (output, _trimmed_messages) = self
            .worker
            .generate(system_prompt, llama_messages, max_tokens, use_grammar)
            .await
            .map_err(LlmError::from)?;
        Ok((output, conversation))
    }

    async fn clear_conversation(&self) -> Result<(), LlmError> {
        self.worker.clear_conversation().await.map_err(LlmError::from)
    }

    async fn trim_context(&self, keep_last_n: usize) -> Result<(), LlmError> {
        self.worker.trim_context(keep_last_n).await.map_err(LlmError::from)
    }

    async fn reload_model(&self, config: LlmBackendConfig) -> Result<(), LlmError> {
        match config {
            LlmBackendConfig::Local(llm_config) => {
                debug!("LocalLlmBackend: reloading model from {}", llm_config.model_path);
                self.worker.reload_model(llm_config).await.map_err(LlmError::from)
            }
            LlmBackendConfig::Ollama(_) => Err(LlmError::BackendInit("Cannot reload Ollama config on LocalLlmBackend".to_string())),
        }
    }

    async fn update_context_config(&self, context_config: ContextConfig) -> Result<(), LlmError> {
        self.worker.update_context_config(context_config).await.map_err(LlmError::from)
    }

    fn max_tokens(&self) -> usize {
        self.worker.config().max_tokens
    }

    fn set_max_tokens(&self, max_tokens: usize) {
        self.worker.set_max_tokens(max_tokens);
    }

    fn backend_type(&self) -> LlmBackendType {
        LlmBackendType::Local
    }

    fn resource_report(&self, last_tool_calls: Vec<String>) -> LlmResourceReport {
        let cfg = self.worker.config();
        LlmResourceReport {
            backend_type: "local".to_string(),
            model_path: Some(cfg.model_path.clone()),
            n_ctx: Some(cfg.n_ctx),
            n_batch: Some(cfg.n_batch),
            max_tokens: Some(cfg.max_tokens),
            temperature: Some(cfg.temperature),
            top_k: Some(cfg.top_k),
            top_p: Some(cfg.top_p),
            n_threads: Some(cfg.n_threads),
            context_overflow_threshold: Some(cfg.context_overflow_threshold),
            n_gpu_layers: Some(cfg.gpu_config.n_gpu_layers),
            rolling_window_keep_last: Some(cfg.context_config.rolling_window_keep_last),
            context_keep_ratio: Some(cfg.context_config.context_keep_ratio),
            min_preserve_tokens: Some(cfg.context_config.min_preserve_tokens),
            ollama_url: None,
            ollama_model: None,
            last_tool_calls,
        }
    }
}
