use crate::config::LlmConfig;
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
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::oneshot;
use tracing::debug;
use tracing::info;
use tracing::warn;

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
    /// The worker channel was closed.
    #[error("Worker channel closed")]
    ChannelClosed,
}

/// LLM inference engine wrapping a llama.cpp model.
/// Created once at service startup. Shared via `Arc`.
pub struct LlmInferenceEngine {
    backend: LlamaBackend,
    model: LlamaModel,
    config: LlmConfig,
}

impl LlmInferenceEngine {
    /// Loads the LLM model from the configured GGUF file with GPU acceleration.
    ///
    /// GPU backend selection happens at compile time via cargo features
    /// (e.g. `llm-vulkan` enables `llama-cpp-4/vulkan`). The `n_gpu_layers`
    /// parameter controls how many model layers are offloaded to the GPU.
    pub fn load(config: &LlmConfig) -> Result<Self, LlmError> {
        let backend = llama_cpp_4::llama_backend::LlamaBackend::init().map_err(|error| LlmError::BackendInit(error.to_string()))?;
        debug!("LLM: backend initialized with {:?}", config.gpu_config.backend);

        let mut model_params = LlamaModelParams::default();

        if config.gpu_config.n_gpu_layers != 0 {
            model_params = model_params.with_n_gpu_layers(config.gpu_config.n_gpu_layers as u32);
            info!("LLM: GPU layer offloading enabled: {} layers", config.gpu_config.n_gpu_layers);
        }

        #[cfg(feature = "llm-vulkan")]
        {
            info!("LLM: Vulkan GPU backend compiled in");
        }

        #[cfg(not(feature = "llm-vulkan"))]
        {
            if config.gpu_config.n_gpu_layers != 0 {
                warn!("LLM: n_gpu_layers={} but no GPU feature compiled in — falling back to CPU", config.gpu_config.n_gpu_layers);
            }
            debug!("LLM: CPU-only backend (no GPU feature compiled in)");
        }

        let model = LlamaModel::load_from_file(&backend, &config.model_path, &model_params).map_err(|error| LlmError::ModelLoad(error.to_string()))?;

        debug!("LLM: model loaded from {}", config.model_path);
        info!("LLM: Model loaded with {} context window, {} batch size", config.n_ctx, config.n_batch);

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
    pub fn create_session(&self) -> Result<LlmSession<'_>, LlmError> {
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

        Ok(LlmSession {
            ctx,
            batch,
            sampler,
            n_cur: 0,
            n_batch: self.config.n_batch as usize,
        })
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
    n_batch: usize,
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

        // 4. Feed delta tokens in chunks of n_batch to handle large prompts.
        let batch_size = self.n_batch;
        for chunk_start in (0..delta_tokens.len()).step_by(batch_size) {
            let chunk_end = (chunk_start + batch_size).min(delta_tokens.len());
            let chunk = &delta_tokens[chunk_start..chunk_end];

            self.batch.clear();
            for (index, &token) in chunk.iter().enumerate() {
                let absolute_index = chunk_start + index;
                let is_last = absolute_index == delta_tokens.len() - 1;
                self.batch
                    .add(token, self.n_cur + absolute_index as i32, &[0], is_last)
                    .map_err(|error| LlmError::Decode(error.to_string()))?;
            }

            self.ctx.decode(&mut self.batch).map_err(|error| LlmError::Decode(error.to_string()))?;
        }
        debug!("LLM: delta batch decoded ({} tokens)", delta_tokens.len());

        self.n_cur += delta_tokens.len() as i32;

        // 5. Autoregressive generation loop.
        let mut generated_tokens: Vec<LlamaToken> = Vec::with_capacity(max_tokens);
        let mut line_buffer: Vec<u8> = Vec::new();

        for _ in 0..max_tokens {
            let token = self.sampler.sample(&self.ctx, -1);
            self.sampler.accept(token);

            if model.is_eog_token(token) {
                debug!("LLM: end-of-generation token produced");
                break;
            }

            generated_tokens.push(token);

            // Stream: detokenize token and log complete lines.
            let token_bytes = model
                .token_to_bytes(token, Special::Tokenize)
                .map_err(|error| LlmError::Detokenize(error.to_string()))?;
            line_buffer.extend_from_slice(&token_bytes);
            while let Some(nl_pos) = line_buffer.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = line_buffer.drain(..=nl_pos).collect();
                let line_str = String::from_utf8_lossy(&line).trim_end().to_string();
                if !line_str.is_empty() {
                    debug!("LLM stream: {}", line_str);
                }
            }

            self.batch.clear();
            self.batch
                .add(token, self.n_cur, &[0], true)
                .map_err(|error| LlmError::Decode(error.to_string()))?;

            self.ctx.decode(&mut self.batch).map_err(|error| LlmError::Decode(error.to_string()))?;

            self.n_cur += 1;
        }

        // Flush remaining partial line.
        if !line_buffer.is_empty() {
            let remaining = String::from_utf8_lossy(&line_buffer).trim().to_string();
            if !remaining.is_empty() {
                debug!("LLM stream: {}", remaining);
            }
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

    /// Shifts the KV cache context by removing the oldest `tokens_to_remove` tokens
    /// and shifting remaining tokens forward, preserving the recent context.
    ///
    /// Uses llama-cpp-4's native KV-cache operations:
    /// 1. `clear_kv_cache_seq` — removes tokens in range [0, tokens_to_remove)
    /// 2. `kv_cache_seq_add` — shifts remaining token positions by -tokens_to_remove
    ///
    /// Returns `Ok(true)` if the shift succeeded, `Ok(false)` if the backend
    /// does not support shifting (caller should fall back to full reset).
    pub fn shift_context(&mut self, tokens_to_remove: usize) -> Result<bool, LlmError> {
        if tokens_to_remove == 0 {
            return Ok(true);
        }

        if !self.ctx.memory_can_shift() {
            debug!("LLM: backend does not support KV cache shifting");
            return Ok(false);
        }

        let seq_id: i32 = 0;
        let p0: u32 = 0;
        let p1: u32 = tokens_to_remove as u32;

        let removed = self
            .ctx
            .clear_kv_cache_seq(Some(seq_id as u32), Some(p0), Some(p1))
            .map_err(|error| LlmError::Decode(format!("KV cache clear failed: {error}")))?;

        if !removed {
            debug!("LLM: partial sequence removal failed, falling back to full reset");
            return Ok(false);
        }

        // Shift remaining token positions backward by tokens_to_remove
        self.ctx
            .kv_cache_seq_add(seq_id, None, None, -(tokens_to_remove as i32))
            .map_err(|error| LlmError::Decode(format!("KV cache shift failed: {error}")))?;

        self.n_cur -= tokens_to_remove as i32;
        debug!("LLM: native context shift removed {} tokens, n_cur now {}", tokens_to_remove, self.n_cur);

        Ok(true)
    }

    /// Clears the entire KV cache and resets the token position.
    ///
    /// This is used for selective cache clearing when the conversation
    /// history should be discarded but the model weights remain in memory.
    pub fn clear_kv_cache(&mut self) {
        self.ctx.clear_kv_cache();
        self.n_cur = 0;
        debug!("LLM: KV cache cleared, n_cur reset to 0");
    }

    /// Returns the current token position in the KV cache.
    #[must_use]
    pub fn n_cur(&self) -> i32 {
        self.n_cur
    }
}

// ============================================================================
// L0: Persistent LLM Worker Thread
// ============================================================================

/// Commands sent from the async service to the LLM worker thread.
enum LlmWorkerCommand {
    /// Generate a completion from the system prompt and conversation.
    /// Uses rolling window trimming when context overflows instead of full reset.
    Generate {
        system_prompt: String,
        conversation: Vec<LlamaChatMessage>,
        max_tokens: usize,
        response_tx: oneshot::Sender<Result<(String, Vec<LlamaChatMessage>), LlmError>>,
    },
    /// Clear conversation history while preserving KV cache and model weights.
    ClearConversation { response_tx: oneshot::Sender<Result<(), LlmError>> },
    /// Trim context to keep only the last `keep_last_n` tokens via native KV-cache shift.
    TrimContext {
        keep_last_n: usize,
        response_tx: oneshot::Sender<Result<(), LlmError>>,
    },
    /// Graceful shutdown.
    Shutdown,
}

/// Handle to the LLM worker thread. Owned by the voice assistant service.
/// Send + Sync safe: communicates via channels, holds no !Send types.
pub struct LlmWorker {
    sender: Arc<Mutex<std::sync::mpsc::Sender<LlmWorkerCommand>>>,
    handle: Option<std::thread::JoinHandle<()>>,
    config: LlmConfig,
}

impl LlmWorker {
    /// Spawns the dedicated LLM worker thread.
    /// The engine is moved into the thread and never shared.
    pub fn spawn(engine: LlmInferenceEngine) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel::<LlmWorkerCommand>();
        let config = engine.config().clone();

        let handle = std::thread::spawn(move || {
            run_worker(engine, receiver);
        });

        Self {
            sender: Arc::new(Mutex::new(sender)),
            handle: Some(handle),
            config,
        }
    }

    /// Returns a reference to the engine configuration.
    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    /// Sends a generate command to the worker and awaits the response.
    pub async fn generate(
        &self,
        system_prompt: &str,
        conversation: Vec<LlamaChatMessage>,
        max_tokens: usize,
    ) -> Result<(String, Vec<LlamaChatMessage>), LlmError> {
        let (response_tx, response_rx) = oneshot::channel();
        let command = LlmWorkerCommand::Generate {
            system_prompt: system_prompt.to_string(),
            conversation,
            max_tokens,
            response_tx,
        };
        {
            let sender = self.sender.lock().map_err(|_| LlmError::ChannelClosed)?;
            sender.send(command).map_err(|_| LlmError::ChannelClosed)?;
        }
        response_rx.await.map_err(|_| LlmError::ChannelClosed)?
    }

    /// Clears the conversation history (KV cache) while keeping model weights in memory.
    ///
    /// This is cheaper than a full reset because the model is not reloaded.
    /// The KV cache is cleared and the token position is reset to zero.
    pub async fn clear_conversation(&self) -> Result<(), LlmError> {
        let (response_tx, response_rx) = oneshot::channel();
        let command = LlmWorkerCommand::ClearConversation { response_tx };
        {
            let sender = self.sender.lock().map_err(|_| LlmError::ChannelClosed)?;
            sender.send(command).map_err(|_| LlmError::ChannelClosed)?;
        }
        response_rx.await.map_err(|_| LlmError::ChannelClosed)?
    }

    /// Trims the context to keep only the last `keep_last_n` tokens.
    ///
    /// Uses native KV-cache shifting when the backend supports it.
    /// Falls back to a full cache clear if shifting is not available.
    pub async fn trim_context(&self, keep_last_n: usize) -> Result<(), LlmError> {
        let (response_tx, response_rx) = oneshot::channel();
        let command = LlmWorkerCommand::TrimContext { keep_last_n, response_tx };
        {
            let sender = self.sender.lock().map_err(|_| LlmError::ChannelClosed)?;
            sender.send(command).map_err(|_| LlmError::ChannelClosed)?;
        }
        response_rx.await.map_err(|_| LlmError::ChannelClosed)?
    }
}

impl Drop for LlmWorker {
    fn drop(&mut self) {
        if let Ok(sender) = self.sender.lock() {
            let _ = sender.send(LlmWorkerCommand::Shutdown);
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Main loop of the LLM worker thread.
/// Owns the LlmInferenceEngine and an optional LlmSession (KV-cache).
fn run_worker(engine: LlmInferenceEngine, receiver: std::sync::mpsc::Receiver<LlmWorkerCommand>) {
    let mut session: Option<LlmSession<'_>> = None;
    let mut last_system_prompt: Option<String> = None;

    while let Ok(command) = receiver.recv() {
        match command {
            LlmWorkerCommand::Generate {
                system_prompt,
                conversation,
                max_tokens,
                response_tx,
            } => {
                let result = handle_generate(&engine, &mut session, &mut last_system_prompt, &system_prompt, &conversation, max_tokens);
                let _ = response_tx.send(result);
            }
            LlmWorkerCommand::ClearConversation { response_tx } => {
                debug!("LLM worker: clearing conversation (KV cache)");
                if let Some(sess) = session.as_mut() {
                    sess.clear_kv_cache();
                }
                last_system_prompt = None;
                let _ = response_tx.send(Ok(()));
            }
            LlmWorkerCommand::TrimContext { keep_last_n, response_tx } => {
                let result = if let Some(sess) = session.as_mut() {
                    let current_n_cur = sess.n_cur() as usize;
                    if current_n_cur <= keep_last_n {
                        debug!("LLM worker: trim_context no-op (current {} <= keep {})", current_n_cur, keep_last_n);
                        Ok(())
                    } else {
                        let tokens_to_remove = current_n_cur - keep_last_n;
                        match sess.shift_context(tokens_to_remove) {
                            Ok(true) => {
                                debug!("LLM worker: trim_context shifted {} tokens", tokens_to_remove);
                                Ok(())
                            }
                            Ok(false) => {
                                debug!("LLM worker: trim_context shift unsupported, clearing KV cache");
                                sess.clear_kv_cache();
                                Ok(())
                            }
                            Err(error) => {
                                warn!("LLM worker: trim_context shift failed: {}, clearing KV cache", error);
                                sess.clear_kv_cache();
                                Ok(())
                            }
                        }
                    }
                } else {
                    Ok(())
                };
                let _ = response_tx.send(result);
            }
            LlmWorkerCommand::Shutdown => {
                debug!("LLM worker: shutting down");
                break;
            }
        }
    }
}

/// Handles a generate command with KV-cache reuse and overflow detection.
/// Uses exact tokenization (model.str_to_token) instead of char heuristics
/// for precise context overflow detection.
///
/// When context overflows, implements rolling window trimming: instead of
/// discarding the entire conversation, trims the oldest messages to fit
/// within the context window, preserving the most recent context.
fn handle_generate<'a>(
    engine: &'a LlmInferenceEngine,
    session: &mut Option<LlmSession<'a>>,
    last_system_prompt: &mut Option<String>,
    system_prompt: &str,
    conversation: &[LlamaChatMessage],
    max_tokens: usize,
) -> Result<(String, Vec<LlamaChatMessage>), LlmError> {
    let prompt_changed = last_system_prompt.as_ref().is_some_and(|prev| prev != system_prompt);

    let n_ctx = engine.config().n_ctx as usize;
    let context_config = &engine.config().context_config;
    let current_n_cur = session.as_ref().map(|s| s.n_cur as usize).unwrap_or(0);

    // Exact token count via the model's native tokenizer.
    let mut all_messages = vec![LlamaChatMessage::new("system".to_string(), system_prompt.to_string()).map_err(|e| LlmError::ChatMessage(e.to_string()))?];
    all_messages.extend(conversation.iter().cloned());
    let formatted_prompt = engine
        .model()
        .apply_chat_template(None, &all_messages, true)
        .map_err(|e| LlmError::ApplyChatTemplate(e.to_string()))?;
    let prompt_tokens = engine
        .model()
        .str_to_token(&formatted_prompt, AddBos::Always)
        .map_err(|e| LlmError::Tokenize(e.to_string()))?;
    let exact_token_count = prompt_tokens.len();

    let overflow_threshold = (n_ctx as f32 * engine.config().context_overflow_threshold) as usize;
    let needs_overflow_reset = current_n_cur + exact_token_count > overflow_threshold;

    // Rolling window trimming: when overflow is detected and the system prompt
    // hasn't changed, trim the oldest conversation messages to fit within the
    // context window instead of discarding everything.
    let mut effective_conversation = conversation.to_vec();

    if needs_overflow_reset && !prompt_changed && !conversation.is_empty() {
        let target_threshold = (n_ctx as f64 * (1.0 - context_config.context_keep_ratio)) as usize;

        let mut trimmed = conversation.to_vec();
        while trimmed.len() > 2 {
            let mut test_messages =
                vec![LlamaChatMessage::new("system".to_string(), system_prompt.to_string()).map_err(|e| LlmError::ChatMessage(e.to_string()))?];
            test_messages.extend(trimmed.iter().cloned());
            let test_prompt = engine
                .model()
                .apply_chat_template(None, &test_messages, true)
                .map_err(|e| LlmError::ApplyChatTemplate(e.to_string()))?;
            let test_tokens = engine
                .model()
                .str_to_token(&test_prompt, AddBos::Always)
                .map_err(|e| LlmError::Tokenize(e.to_string()))?;

            if test_tokens.len() <= target_threshold {
                break;
            }
            trimmed.remove(0);
        }

        if trimmed.len() < conversation.len() {
            debug!(
                "LLM worker: rolling window trimmed {} -> {} messages (target: {} tokens)",
                conversation.len(),
                trimmed.len(),
                target_threshold
            );
            effective_conversation = trimmed;
        }
    }

    // Recompute token count with the (possibly trimmed) conversation.
    if effective_conversation.len() != conversation.len() {
        all_messages = vec![LlamaChatMessage::new("system".to_string(), system_prompt.to_string()).map_err(|e| LlmError::ChatMessage(e.to_string()))?];
        all_messages.extend(effective_conversation.iter().cloned());
    }

    // Determine if we still need a session reset after trimming.
    let formatted_prompt = engine
        .model()
        .apply_chat_template(None, &all_messages, true)
        .map_err(|e| LlmError::ApplyChatTemplate(e.to_string()))?;
    let prompt_tokens = engine
        .model()
        .str_to_token(&formatted_prompt, AddBos::Always)
        .map_err(|e| LlmError::Tokenize(e.to_string()))?;
    let trimmed_token_count = prompt_tokens.len();
    let still_overflows = current_n_cur + trimmed_token_count > overflow_threshold;

    if session.is_none() || prompt_changed {
        if prompt_changed {
            debug!("LLM worker: resetting session (system prompt changed)");
        } else {
            debug!("LLM worker: creating new session (first call)");
        }
        *session = Some(engine.create_session()?);
        *last_system_prompt = Some(system_prompt.to_string());
    } else if still_overflows {
        // Attempt native KV-cache context shift before falling back to full reset.
        // This preserves the recent context in the KV cache without re-evaluating all tokens.
        let ratio_preserve = (current_n_cur as f64 * context_config.context_keep_ratio) as usize;
        let preserve_tokens = ratio_preserve.max(context_config.min_preserve_tokens);
        let tokens_to_remove = current_n_cur.saturating_sub(preserve_tokens);

        if tokens_to_remove > 0 {
            if let Some(sess) = session.as_mut() {
                match sess.shift_context(tokens_to_remove) {
                    Ok(true) => {
                        debug!(
                            "LLM worker: native KV-cache shift removed {} tokens (preserve ratio: {})",
                            tokens_to_remove, context_config.context_keep_ratio
                        );
                    }
                    Ok(false) => {
                        debug!("LLM worker: KV-cache shift unsupported, falling back to full session reset");
                        *session = Some(engine.create_session()?);
                        *last_system_prompt = Some(system_prompt.to_string());
                    }
                    Err(error) => {
                        warn!("LLM worker: KV-cache shift failed: {}, falling back to full session reset", error);
                        *session = Some(engine.create_session()?);
                        *last_system_prompt = Some(system_prompt.to_string());
                    }
                }
            }
        } else {
            debug!("LLM worker: resetting session (no tokens to shift)");
            *session = Some(engine.create_session()?);
            *last_system_prompt = Some(system_prompt.to_string());
        }
    }

    let session = session.as_mut().expect("session should be initialized");
    let output = session.generate(engine.model(), system_prompt, &effective_conversation, max_tokens)?;
    Ok((output, effective_conversation))
}
