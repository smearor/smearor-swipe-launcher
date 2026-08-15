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
use std::sync::RwLock;
use tokio::sync::oneshot;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Minimum pattern length (in tokens) to consider for repetition detection.
const MIN_PATTERN_LEN: usize = 8;

/// Number of consecutive repetitions required to trigger early abort.
const REQUIRED_REPETITIONS: usize = 3;

/// Strips leading numbering from a line (e.g. "5. Only if..." -> "Only if...").
/// This normalizes lines for repetition detection when the model echoes hint
/// text with incrementing numbers.
fn strip_leading_number(line: &str) -> String {
    let trimmed = line.trim_start();
    let mut chars = trimmed.chars().peekable();
    let mut digit_count = 0;
    while chars.peek().map_or(false, |c| c.is_ascii_digit()) {
        chars.next();
        digit_count += 1;
    }
    if digit_count > 0 {
        if chars.peek() == Some(&'.') {
            chars.next();
            if chars.peek() == Some(&' ') {
                chars.next();
            }
            return chars.collect();
        }
    }
    trimmed.to_string()
}

/// GBNF grammar that constrains the LLM to emit a single JSON object.
///
/// The original strict ReAct grammar caused llama.cpp's grammar sampler to
/// empty its stack after the model emitted the prefix `{"` on both Gemma and
/// Qwen models. Replacing it with the community-standard JSON object grammar
/// from llama.cpp's own examples avoids that crash. The expected ReAct keys
/// ("tool", "final_answer", "clarify") are validated by the Rust parser in
/// `react.rs` after generation.
const REACT_GBNF_GRAMMAR: &str = r#"
root   ::= object
object ::= "{" ws "}" | "{" ws pair ("," ws pair)* ws "}"
pair   ::= string ws ":" ws value
value  ::= object | array | string | number | "true" | "false" | "null"
array  ::= "[" ws "]" | "[" ws value ("," ws value)* ws "]"
string ::= "\"" ([^"\\] | "\\" (["\\/bfnrt] | "u" [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F]))* "\""
number ::= ("-"? ([0-9] | [1-9] [0-9]*)) ("." [0-9]+)? ([eE] [-+]? [0-9]+)?
ws     ::= [ \t\n]*
"#;

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
    /// The generation entered a repetition loop (same token sequence repeated).
    #[error("Repetition loop detected: {0} tokens repeated {1} times")]
    RepetitionLoop(usize, usize),
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
    /// GPU backend selection happens at **compile time** via cargo features
    /// (e.g. `llm-vulkan` enables `llama-cpp-4/vulkan`, `llm-hipblas` enables
    /// `llama-cpp-4/hip`). The `n_gpu_layers` parameter controls how many
    /// model layers are offloaded to the GPU.
    pub fn load(config: &LlmConfig) -> Result<Self, LlmError> {
        let backend = llama_cpp_4::llama_backend::LlamaBackend::init().map_err(|error| LlmError::BackendInit(error.to_string()))?;

        #[cfg(feature = "llm-hipblas")]
        {
            info!("LLM: backend initialized (compiled: HIPBLAS)");
        }
        #[cfg(feature = "llm-vulkan")]
        {
            info!("LLM: backend initialized (compiled: Vulkan)");
        }
        #[cfg(not(any(feature = "llm-vulkan", feature = "llm-hipblas")))]
        {
            info!("LLM: backend initialized (compiled: CPU-only)");
        }

        let mut model_params = LlamaModelParams::default();

        if config.gpu_config.n_gpu_layers != 0 {
            model_params = model_params.with_n_gpu_layers(config.gpu_config.n_gpu_layers as u32);
            info!("LLM: GPU layer offloading enabled: {} layers", config.gpu_config.n_gpu_layers);
        }

        #[cfg(not(any(feature = "llm-vulkan", feature = "llm-hipblas")))]
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

    /// Updates the context configuration at runtime without reloading the model.
    /// This is used for live-tuning parameters like `rolling_window_keep_last`.
    pub fn update_context_config(&mut self, context_config: crate::config::ContextConfig) {
        self.config.context_config = context_config;
    }

    /// Reloads the model from a different GGUF file, reusing the existing backend.
    ///
    /// The caller must drop any active `LlmSession` before calling this,
    /// because the old `LlamaModel` is replaced and existing contexts
    /// (KV caches) are invalidated.
    pub fn reload(&mut self, config: &LlmConfig) -> Result<(), LlmError> {
        let mut model_params = LlamaModelParams::default();

        if config.gpu_config.n_gpu_layers != 0 {
            model_params = model_params.with_n_gpu_layers(config.gpu_config.n_gpu_layers as u32);
            info!("LLM: GPU layer offloading enabled: {} layers", config.gpu_config.n_gpu_layers);
        }

        #[cfg(not(any(feature = "llm-vulkan", feature = "llm-hipblas")))]
        {
            if config.gpu_config.n_gpu_layers != 0 {
                warn!("LLM: n_gpu_layers={} but no GPU feature compiled in — falling back to CPU", config.gpu_config.n_gpu_layers);
            }
        }

        self.model = LlamaModel::load_from_file(&self.backend, &config.model_path, &model_params).map_err(|error| LlmError::ModelLoad(error.to_string()))?;

        debug!("LLM: model reloaded from {}", config.model_path);
        info!("LLM: Model reloaded with {} context window, {} batch size", config.n_ctx, config.n_batch);

        self.config = config.clone();
        Ok(())
    }

    /// Creates a sampler chain with optional grammar-based JSON enforcement.
    ///
    /// When `use_grammar` is true, a GBNF grammar sampler is prepended to the
    /// chain that physically constrains the LLM to output only valid JSON
    /// matching the ReAct format (`{"tool": ...}` or `{"final_answer": ...}`).
    /// This eliminates parse errors from small models producing free-form text.
    pub fn create_sampler(&self, use_grammar: bool) -> LlamaSampler {
        let base_samplers = [
            LlamaSampler::temp(self.config.temperature),
            LlamaSampler::top_k(self.config.top_k),
            LlamaSampler::top_p(self.config.top_p, 1),
            LlamaSampler::dist(0),
        ];

        if !use_grammar {
            return LlamaSampler::chain_simple(base_samplers);
        }

        // Verify the grammar string contains the root rule and is safe to pass
        // to the C++ grammar parser. This guards against packaging errors that
        // would otherwise panic inside llama.cpp.
        if !REACT_GBNF_GRAMMAR.contains("root") || !REACT_GBNF_GRAMMAR.contains("::=") || REACT_GBNF_GRAMMAR.contains('\0') {
            warn!("LLM: ReAct GBNF grammar is missing the root rule or contains null bytes; falling back to no-grammar sampler");
            return LlamaSampler::chain_simple(base_samplers);
        }

        debug!("LLM: creating sampler with GBNF grammar enforcement");
        LlamaSampler::chain_simple([
            LlamaSampler::grammar(&self.model, REACT_GBNF_GRAMMAR, "root"),
            LlamaSampler::temp(self.config.temperature),
            LlamaSampler::top_k(self.config.top_k),
            LlamaSampler::top_p(self.config.top_p, 1),
            LlamaSampler::dist(0),
        ])
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

        let sampler = self.create_sampler(false);

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
        let mut generated_lines: Vec<String> = Vec::new();

        // Text-based stop markers for models that generate chat-template tokens
        // (e.g. `<|im_end|>`) as regular BPE text rather than special tokens.
        // Gemma4-heretic uses a ChatML template but lacks ChatML special tokens
        // in its vocabulary, so `is_eog_token` cannot catch these.
        const TEXT_STOP_MARKERS: &[&str] = &["<|im_end|>", "<|im_start|>"];
        let mut text_output: String = String::with_capacity(max_tokens * 4);

        for _ in 0..max_tokens {
            let token = self.sampler.sample(&self.ctx, -1);
            self.sampler.accept(token);

            if model.is_eog_token(token) {
                debug!("LLM: end-of-generation token produced");
                break;
            }

            generated_tokens.push(token);

            // Text-based stop sequence: check if any chat-template marker
            // appeared in the accumulated output. If so, truncate at the
            // marker position and stop generation.
            let token_bytes = model
                .token_to_bytes(token, Special::Tokenize)
                .map_err(|error| LlmError::Detokenize(error.to_string()))?;
            text_output.push_str(&String::from_utf8_lossy(&token_bytes));
            if let Some(marker) = TEXT_STOP_MARKERS.iter().find(|m| text_output.contains(*m)) {
                let truncate_pos = text_output.find(marker).unwrap_or(text_output.len());
                let output = text_output[..truncate_pos].to_string();
                debug!("LLM: text-based stop marker '{}' detected, truncating output to {} chars", marker, output.len());
                return Ok(output);
            }

            // Repetition loop detection: check if the tail of generated_tokens
            // consists of a repeating pattern of length L.
            // For each candidate pattern length L from MIN_PATTERN_LEN to len/REQUIRED_REPETITIONS,
            // check if the last L*REQUIRED_REPETITIONS tokens are L repeated REQUIRED_REPETITIONS times.
            let gen_len = generated_tokens.len();
            if gen_len >= MIN_PATTERN_LEN * REQUIRED_REPETITIONS {
                let max_pattern_len = gen_len / REQUIRED_REPETITIONS;
                for pattern_len in MIN_PATTERN_LEN..=max_pattern_len {
                    let repeat_len = pattern_len * REQUIRED_REPETITIONS;
                    if gen_len < repeat_len {
                        continue;
                    }
                    let tail_start = gen_len - repeat_len;
                    let pattern = &generated_tokens[tail_start..tail_start + pattern_len];
                    let mut is_repeating = true;
                    for rep in 1..REQUIRED_REPETITIONS {
                        let chunk_start = tail_start + rep * pattern_len;
                        if &generated_tokens[chunk_start..chunk_start + pattern_len] != pattern {
                            is_repeating = false;
                            break;
                        }
                    }
                    if is_repeating {
                        debug!(
                            "LLM: repetition loop detected: {} tokens repeated {} times, aborting generation",
                            pattern_len, REQUIRED_REPETITIONS
                        );
                        // Remove the repeated tail from generated tokens.
                        generated_tokens.truncate(tail_start);
                        // Detokenize the partial output before the repetition.
                        let mut all_bytes: Vec<u8> = Vec::with_capacity(generated_tokens.len() * 4);
                        for tok in &generated_tokens {
                            let tok_bytes = model
                                .token_to_bytes(*tok, Special::Tokenize)
                                .map_err(|error| LlmError::Detokenize(error.to_string()))?;
                            all_bytes.extend_from_slice(&tok_bytes);
                        }
                        let output = String::from_utf8_lossy(&all_bytes).into_owned();
                        debug!("LLM: returning partial output ({} chars) before repetition loop", output.len());
                        return Ok(output);
                    }
                }
            }

            // Stream: use the bytes already detokenized above for line logging.
            line_buffer.extend_from_slice(&token_bytes);
            while let Some(nl_pos) = line_buffer.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = line_buffer.drain(..=nl_pos).collect();
                let line_str = String::from_utf8_lossy(&line).trim_end().to_string();
                if !line_str.is_empty() {
                    debug!("LLM stream: {}", line_str);
                    generated_lines.push(line_str.clone());

                    // Line-based repetition detection: strip leading numbering
                    // (e.g. "5. Only if..." -> "Only if...") and check if the
                    // same normalized line content appears 3+ times consecutively.
                    let normalized = strip_leading_number(&line_str);
                    if !normalized.is_empty() {
                        let consecutive_count = generated_lines
                            .iter()
                            .rev()
                            .map(|l| strip_leading_number(l))
                            .take_while(|l| l == &normalized)
                            .count();
                        if consecutive_count >= REQUIRED_REPETITIONS {
                            debug!("LLM: line repetition detected: '{}' repeated {} times, aborting generation", normalized, consecutive_count);
                            // Remove the repeated lines from generated tokens by
                            // re-detokenizing only the lines before the repetition.
                            let lines_to_keep = generated_lines.len() - consecutive_count;
                            let kept_lines: Vec<&str> = generated_lines.iter().take(lines_to_keep).map(|s| s.as_str()).collect();
                            let output = kept_lines.join("\n");
                            debug!("LLM: returning partial output ({} chars) before line repetition", output.len());
                            return Ok(output);
                        }
                    }
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

        // 6. Return the accumulated text output (already detokenized incrementally).
        debug!("LLM: generated {} tokens, {} chars", generated_tokens.len(), text_output.len());
        Ok(text_output)
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
    /// When `use_grammar` is true, GBNF grammar enforcement constrains output
    /// to the ReAct JSON format (`{"tool": ...}` or `{"final_answer": ...}`).
    Generate {
        system_prompt: String,
        conversation: Vec<LlamaChatMessage>,
        max_tokens: usize,
        use_grammar: bool,
        response_tx: oneshot::Sender<Result<(String, Vec<LlamaChatMessage>), LlmError>>,
    },
    /// Clear conversation history while preserving KV cache and model weights.
    ClearConversation { response_tx: oneshot::Sender<Result<(), LlmError>> },
    /// Trim context to keep only the last `keep_last_n` tokens via native KV-cache shift.
    TrimContext {
        keep_last_n: usize,
        response_tx: oneshot::Sender<Result<(), LlmError>>,
    },
    /// Reload the model from a new GGUF file, reusing the existing backend.
    /// The session (KV cache) is invalidated and must be recreated.
    ReloadModel {
        config: LlmConfig,
        response_tx: oneshot::Sender<Result<(), LlmError>>,
    },
    /// Update context configuration at runtime without reloading the model.
    UpdateContextConfig {
        context_config: crate::config::ContextConfig,
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
    config: Arc<RwLock<LlmConfig>>,
}

impl LlmWorker {
    /// Spawns the dedicated LLM worker thread.
    /// The engine is moved into the thread and never shared.
    pub fn spawn(engine: LlmInferenceEngine) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel::<LlmWorkerCommand>();
        let config = Arc::new(RwLock::new(engine.config().clone()));

        let handle = std::thread::spawn(move || {
            run_worker(engine, receiver);
        });

        Self {
            sender: Arc::new(Mutex::new(sender)),
            handle: Some(handle),
            config,
        }
    }

    /// Returns a clone of the engine configuration.
    pub fn config(&self) -> LlmConfig {
        self.config.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Updates `max_tokens` in the runtime config without reloading the model.
    /// The value is read by the ReAct loop at the start of each iteration,
    /// so changes take effect on the next `generate` call.
    pub fn set_max_tokens(&self, max_tokens: usize) {
        if let Ok(mut config) = self.config.write() {
            config.max_tokens = max_tokens;
        }
    }

    /// Sends a generate command to the worker and awaits the response.
    ///
    /// When `use_grammar` is true, a GBNF grammar sampler constrains the LLM
    /// output to the ReAct JSON format, eliminating parse errors from
    /// free-form text generation.
    pub async fn generate(
        &self,
        system_prompt: &str,
        conversation: Vec<LlamaChatMessage>,
        max_tokens: usize,
        use_grammar: bool,
    ) -> Result<(String, Vec<LlamaChatMessage>), LlmError> {
        let (response_tx, response_rx) = oneshot::channel();
        let command = LlmWorkerCommand::Generate {
            system_prompt: system_prompt.to_string(),
            conversation,
            max_tokens,
            use_grammar,
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

    /// Reloads the LLM model from a new GGUF file at runtime.
    ///
    /// The existing `LlamaBackend` is reused; only the `LlamaModel` is replaced.
    /// The KV cache (session) is invalidated and will be recreated on the next
    /// `Generate` command. If loading the new model fails, the old model remains
    /// active and an error is returned.
    pub async fn reload_model(&self, config: LlmConfig) -> Result<(), LlmError> {
        let (response_tx, response_rx) = oneshot::channel();
        let command = LlmWorkerCommand::ReloadModel {
            config: config.clone(),
            response_tx,
        };
        {
            let sender = self.sender.lock().map_err(|_| LlmError::ChannelClosed)?;
            sender.send(command).map_err(|_| LlmError::ChannelClosed)?;
        }
        let result = response_rx.await.map_err(|_| LlmError::ChannelClosed)?;
        if result.is_ok() {
            if let Ok(mut cfg) = self.config.write() {
                *cfg = config;
            }
        }
        result
    }

    /// Updates the context configuration at runtime without reloading the model.
    ///
    /// This allows live-tuning of `rolling_window_keep_last`, `context_keep_ratio`,
    /// and `min_preserve_tokens` without the expensive model reload cycle.
    pub async fn update_context_config(&self, context_config: crate::config::ContextConfig) -> Result<(), LlmError> {
        let (response_tx, response_rx) = oneshot::channel();
        let command = LlmWorkerCommand::UpdateContextConfig {
            context_config: context_config.clone(),
            response_tx,
        };
        {
            let sender = self.sender.lock().map_err(|_| LlmError::ChannelClosed)?;
            sender.send(command).map_err(|_| LlmError::ChannelClosed)?;
        }
        let result = response_rx.await.map_err(|_| LlmError::ChannelClosed)?;
        if result.is_ok() {
            if let Ok(mut cfg) = self.config.write() {
                cfg.context_config = context_config;
            }
        }
        result
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
fn run_worker(mut engine: LlmInferenceEngine, receiver: std::sync::mpsc::Receiver<LlmWorkerCommand>) {
    let mut session: Option<LlmSession<'_>> = None;
    let mut last_system_prompt: Option<String> = None;

    while let Ok(command) = receiver.recv() {
        match command {
            LlmWorkerCommand::Generate {
                system_prompt,
                conversation,
                max_tokens,
                use_grammar,
                response_tx,
            } => {
                let result = handle_generate(&engine, &mut session, &mut last_system_prompt, &system_prompt, &conversation, max_tokens, use_grammar);
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
            LlmWorkerCommand::ReloadModel { config, response_tx } => {
                session = None;
                last_system_prompt = None;
                debug!("LLM worker: reloading model from {}", config.model_path);
                let result = engine.reload(&config);
                if let Err(ref error) = result {
                    warn!("LLM worker: model reload failed: {error}");
                }
                let _ = response_tx.send(result);
            }
            LlmWorkerCommand::UpdateContextConfig { context_config, response_tx } => {
                debug!(
                    "LLM worker: updating context config (rolling_window_keep_last: {}, keep_ratio: {}, min_preserve: {})",
                    context_config.rolling_window_keep_last, context_config.context_keep_ratio, context_config.min_preserve_tokens
                );
                // Drop the session first — it holds an immutable borrow of engine.
                // The session will be recreated on the next Generate command.
                session = None;
                last_system_prompt = None;
                engine.update_context_config(context_config);
                let _ = response_tx.send(Ok(()));
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
    use_grammar: bool,
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

    // Detect when the new prompt is not a superset of the cached tokens.
    // This happens when a new ReAct loop starts with a different conversation —
    // the KV cache is stale and must be reset to avoid decoding errors.
    let prompt_shrunk = exact_token_count <= current_n_cur;

    let overflow_threshold = (n_ctx as f32 * engine.config().context_overflow_threshold) as usize;
    let needs_overflow_reset = current_n_cur + exact_token_count > overflow_threshold;

    // Rolling window trimming: when overflow is detected and the system prompt
    // hasn't changed, trim the oldest conversation messages to fit within the
    // context window instead of discarding everything.
    let mut effective_conversation = conversation.to_vec();

    if needs_overflow_reset && !prompt_changed && !conversation.is_empty() {
        let target_threshold = (n_ctx as f64 * (1.0 - context_config.context_keep_ratio)) as usize;

        let mut trimmed = conversation.to_vec();
        // Always keep the first message (context with tool schemas) and the
        // last N messages (configurable via rolling_window_keep_last). Trim
        // older conversation history from index 1 to preserve critical context.
        let keep_last = context_config.rolling_window_keep_last;
        let min_messages = keep_last + 1; // +1 for context message at index 0
        while trimmed.len() > min_messages {
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
            trimmed.remove(1);
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

    let trimmed_shrunk = trimmed_token_count <= current_n_cur;
    if session.is_none() {
        debug!("LLM worker: creating new session (first call)");
        *session = Some(engine.create_session()?);
        *last_system_prompt = Some(system_prompt.to_string());
    } else if prompt_changed {
        debug!("LLM worker: resetting session (system prompt changed)");
        *session = Some(engine.create_session()?);
        *last_system_prompt = Some(system_prompt.to_string());
    } else if prompt_shrunk || trimmed_shrunk {
        // Prompt shrank between ReAct iterations (e.g. rolling window trimmed
        // older messages). Clear the KV cache on the existing session instead
        // of allocating a new LlamaContext — much cheaper, same result.
        debug!("LLM worker: clearing KV cache (prompt shrunk: {} <= cached {})", trimmed_token_count, current_n_cur);
        if let Some(sess) = session.as_mut() {
            sess.clear_kv_cache();
        }
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
    session.sampler = engine.create_sampler(use_grammar);
    let output = session.generate(engine.model(), system_prompt, &effective_conversation, max_tokens)?;
    Ok((output, effective_conversation))
}
