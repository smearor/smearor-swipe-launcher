use llama_cpp_4::context::LlamaContext;
use llama_cpp_4::context::params::LlamaContextParams;
use llama_cpp_4::llama_backend::LlamaBackend;
use llama_cpp_4::llama_batch::LlamaBatch;
use llama_cpp_4::model::AddBos;
use llama_cpp_4::model::LlamaChatMessage;
use llama_cpp_4::model::LlamaModel;
use llama_cpp_4::model::Special;
use llama_cpp_4::model::params::LlamaModelParams;
use llama_cpp_4::sampling::LlamaSampler;
use llama_cpp_4::token::LlamaToken;
use std::num::NonZeroU32;
use tracing::debug;

use crate::config::LlmConfig;

/// Errors that can occur during LLM inference.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    /// Failed to initialize the llama.cpp backend.
    #[error("Failed to initialize backend: {0}")]
    BackendInit(String),
    /// Failed to load the model from the GGUF file.
    #[error("Failed to load model: {0}")]
    ModelLoad(String),
    /// Failed to create a context from the model.
    #[error("Failed to create context: {0}")]
    ContextCreate(String),
    /// Failed to apply the chat template to the messages.
    #[error("Failed to apply chat template: {0}")]
    ApplyChatTemplate(String),
    /// Failed to tokenize the input string.
    #[error("Failed to tokenize: {0}")]
    Tokenize(String),
    /// Failed to detokenize the output tokens.
    #[error("Failed to detokenize: {0}")]
    Detokenize(String),
    /// Failed to decode a batch of tokens.
    #[error("Failed to decode batch: {0}")]
    Decode(String),
    /// Failed to create a chat message.
    #[error("Failed to create chat message: {0}")]
    ChatMessage(String),
    /// The generation exceeded the maximum token limit without producing an EOS.
    #[error("Max tokens ({0}) reached")]
    MaxTokensReached(usize),
    /// The LLM engine has not been initialized.
    #[error("LLM engine not initialized")]
    NotInitialized,
}

/// LLM inference engine wrapping a llama.cpp model.
/// Created once at service startup. Shared via `Arc`.
pub struct LlmInferenceEngine {
    backend: LlamaBackend,
    model: LlamaModel,
    config: LlmConfig,
}

impl LlmInferenceEngine {
    /// Loads the LLM model from the configured GGUF file.
    pub fn load(config: &LlmConfig) -> Result<Self, LlmError> {
        let backend = LlamaBackend::init().map_err(|error| LlmError::BackendInit(error.to_string()))?;
        debug!("LLM: backend initialized");

        let model_params = LlamaModelParams::default();
        let model = LlamaModel::load_from_file(&backend, &config.model_path, &model_params).map_err(|error| LlmError::ModelLoad(error.to_string()))?;
        debug!("LLM: model loaded from {}", config.model_path);

        Ok(Self {
            backend,
            model,
            config: config.clone(),
        })
    }

    /// Returns a reference to the loaded model.
    pub fn model(&self) -> &LlamaModel {
        &self.model
    }

    /// Returns a reference to the engine configuration.
    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    /// Creates a new inference session for one pipeline run.
    /// The session holds the `LlamaContext` (with KV cache) and is reused
    /// across all ReAct iterations within a single pipeline run.
    /// Must be called from within `spawn_blocking`.
    pub fn create_session(&self) -> Result<LlmSession, LlmError> {
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(self.config.n_ctx))
            .with_n_batch(self.config.n_batch)
            .with_n_threads(self.config.n_threads)
            .with_n_threads_batch(self.config.n_threads)
            .with_n_seq_max(1);

        let ctx = self
            .model
            .new_context(&self.backend, ctx_params)
            .map_err(|error| LlmError::ContextCreate(error.to_string()))?;
        debug!("LLM: session context created (n_ctx={})", ctx.n_ctx());

        let batch = LlamaBatch::new(self.config.n_batch as usize, 1);

        let sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(self.config.temperature),
            LlamaSampler::top_k(self.config.top_k),
            LlamaSampler::top_p(self.config.top_p, 1),
            LlamaSampler::dist(0),
        ]);

        Ok(LlmSession { ctx, batch, sampler, n_cur: 0 })
    }
}

/// A reusable LLM inference session for one pipeline run.
/// Holds the `LlamaContext` (with KV cache), `LlamaBatch`, and `LlamaSampler`.
/// All fields are `!Send` — the session must live on the `spawn_blocking` thread.
pub struct LlmSession<'a> {
    ctx: LlamaContext<'a>,
    batch: LlamaBatch,
    sampler: LlamaSampler,
    n_cur: i32,
}

impl<'a> LlmSession<'a> {
    /// Generates a completion from the system prompt and conversation history.
    ///
    /// The full conversation (system prompt + all messages) is always passed
    /// through `apply_chat_template` and tokenized. To leverage the KV cache
    /// across ReAct iterations, only the delta tokens (tokens not yet processed
    /// by the context) are fed to the batch.
    pub fn generate(&mut self, model: &LlamaModel, system_prompt: &str, conversation: &[LlamaChatMessage], max_tokens: usize) -> Result<String, LlmError> {
        // 1. Always build the full prompt with system + entire conversation.
        let mut all_messages =
            vec![LlamaChatMessage::new("system".to_string(), system_prompt.to_string()).map_err(|error| LlmError::ChatMessage(error.to_string()))?];
        all_messages.extend(conversation.iter().cloned());

        let prompt = model
            .apply_chat_template(None, &all_messages, true)
            .map_err(|error| LlmError::ApplyChatTemplate(error.to_string()))?;
        debug!("LLM: formatted prompt ({} chars)", prompt.len());

        // 2. Tokenize the full prompt with BOS on first call only.
        let prompt_tokens = model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|error| LlmError::Tokenize(error.to_string()))?;
        debug!("LLM: tokenized to {} tokens total", prompt_tokens.len());

        // 3. Compute the delta: only tokens beyond self.n_cur are new.
        let prev_tokens = self.n_cur as usize;
        if prompt_tokens.len() <= prev_tokens {
            return Err(LlmError::Decode("No new tokens to decode".to_string()));
        }

        let delta_tokens = &prompt_tokens[prev_tokens..];
        debug!("LLM: {} delta tokens ({} already in KV cache)", delta_tokens.len(), prev_tokens);

        // 4. Feed only the delta tokens as a batch.
        self.batch.clear();
        for (index, &token) in delta_tokens.iter().enumerate() {
            let is_last = index == delta_tokens.len() - 1;
            self.batch
                .add(token, self.n_cur + index as i32, &[0], is_last)
                .map_err(|error| LlmError::Decode(error.to_string()))?;
        }

        self.ctx.decode(&mut self.batch).map_err(|error| LlmError::Decode(error.to_string()))?;
        debug!("LLM: delta batch decoded ({} tokens)", delta_tokens.len());

        self.n_cur += delta_tokens.len() as i32;

        // 5. Autoregressive generation loop.
        let mut generated_tokens: Vec<LlamaToken> = Vec::with_capacity(max_tokens);

        for _ in 0..max_tokens {
            let token = self.sampler.sample(&self.ctx, -1);
            self.sampler.accept(token);

            if model.is_eog_token(token) {
                debug!("LLM: end-of-generation token produced");
                break;
            }

            generated_tokens.push(token);

            self.batch.clear();
            self.batch
                .add(token, self.n_cur, &[0], true)
                .map_err(|error| LlmError::Decode(error.to_string()))?;

            self.ctx.decode(&mut self.batch).map_err(|error| LlmError::Decode(error.to_string()))?;

            self.n_cur += 1;
        }

        if generated_tokens.len() >= max_tokens {
            return Err(LlmError::MaxTokensReached(max_tokens));
        }

        // 6. Detokenize the generated tokens.
        let mut all_bytes: Vec<u8> = Vec::with_capacity(generated_tokens.len() * 4);
        for token in &generated_tokens {
            let token_bytes = model
                .token_to_bytes(*token, Special::Tokenize)
                .map_err(|error| LlmError::Detokenize(error.to_string()))?;
            all_bytes.extend_from_slice(&token_bytes);
        }

        let output = String::from_utf8_lossy(&all_bytes).into_owned();

        debug!("LLM: generated {} tokens, {} chars", generated_tokens.len(), output.len());
        Ok(output)
    }
}
