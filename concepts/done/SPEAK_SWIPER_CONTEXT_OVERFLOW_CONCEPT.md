# Persistent LLM Session & KV-Cache Reuse Across Pipeline Runs

This document describes the concept for a **persistent LLM worker thread** that retains the KV cache across multiple voice command pipeline runs, eliminating
redundant pre-fill of the system prompt and significantly reducing CPU latency on resource-constrained devices.

---

## 1. Problem Statement

### Current Architecture

The voice assistant service (`services/voice_assistant`) creates a fresh `LlmSession` (and thus a fresh `LlamaContext` with an empty KV cache) for every
`execute_react_loop` call. This happens in two places:

- `react.rs:86` — initial ReAct loop entry
- `react.rs:169` — recursive continuation after tool response

Each `create_session()` call invokes `model.new_context()`, which allocates a new KV cache buffer of size `n_ctx` tokens. The system prompt (which includes the
full tool catalog, potentially 8000+ characters) is then re-tokenized and re-processed from scratch on every pipeline run.

### Performance Impact

With `qwen2.5-1.5b-instruct` on a 4-thread CPU configuration:

| Scenario                       | Pre-fill Tokens           | Pre-fill Time | Waste                              |
|--------------------------------|---------------------------|---------------|------------------------------------|
| First command                  | ~2000 (system + user)     | ~1.5 s        | None                               |
| Second command (fresh session) | ~2010 (system + new user) | ~1.5 s        | ~1.5 s re-processing system prompt |
| Third command (fresh session)  | ~2010                     | ~1.5 s        | ~1.5 s                             |
| 10th command                   | ~2010                     | ~1.5 s        | ~13.5 s cumulative waste           |

With a persistent session, only the delta (new user message, ~10 tokens) would need processing after the first command:

| Scenario                    | Delta Tokens | Delta Time |
|-----------------------------|--------------|------------|
| First command               | ~2000        | ~1.5 s     |
| Second command (persistent) | ~10          | ~0.01 s    |
| Third command (persistent)  | ~10          | ~0.01 s    |

### Root Cause

`LlmSession<'a>` holds a `LlamaContext<'a>` which is `!Send`. The current architecture uses `tokio::task::spawn_blocking` to run inference on the blocking
thread pool, but Tokio does not guarantee the same thread across multiple `spawn_blocking` calls. Therefore, the session cannot be persisted between calls.

---

## 2. Proposed Solution: Dedicated LLM Worker Thread

### Architecture

```
+---------------------+     Command Channel (mpsc)     +--------------------------+
| Voice Assistant     |------------------------------->| LLM Worker Thread        |
| Service (async)     |                                | (dedicated OS thread)    |
|                     |                                |                          |
| execute_react_loop  |  sends Generate command        | Holds:                   |
| invoke_tool (await) |  awaits oneshot response       |  - LlmInferenceEngine    |
|                     |                                |  - LlmSession (persist)  |
|                     |<-------------------------------|  - n_cur tracking        |
|                     |     Response (oneshot)         |  - Conversation history  |
+---------------------+                                |                          |
                                                       |  On context overflow:   |
                                                       |  reset session           |
                                                       |  (re-create context)     |
                                                       +--------------------------+
```

The worker thread is a dedicated OS thread (not a Tokio blocking pool thread). It owns all `!Send` types and lives for the entire lifetime of the service.
Communication happens via `std::sync::mpsc` (commands) and `tokio::sync::oneshot` (responses).

### Why Not `spawn_blocking`?

`tokio::task::spawn_blocking` runs tasks on a shared thread pool. There is no guarantee that two consecutive `spawn_blocking` calls execute on the same thread.
Since `LlamaContext` is `!Send`, it cannot be moved between threads. A dedicated thread solves this by providing a stable ownership anchor.

---

## 3. Data Model

### Worker Command Enum

```rust
/// Commands sent from the async service to the LLM worker thread.
enum LlmWorkerCommand {
    /// Generate a completion from the system prompt and conversation.
    /// The response is sent back via the oneshot channel.
    Generate {
        /// The system prompt (tool catalog injected).
        system_prompt: String,
        /// The full conversation history (user + assistant messages).
        conversation: Vec<LlamaChatMessage>,
        /// Maximum number of tokens to generate.
        max_tokens: usize,
        /// Response channel: returns (llm_output, updated_conversation).
        response_tx: oneshot::Sender<Result<(String, Vec<LlamaChatMessage>), LlmError>>,
    },
    /// Reset the session (discard KV cache and conversation history).
    /// Called when the tool catalog changes or on explicit reset.
    Reset {
        /// Optional new system prompt to use for the next session.
        /// If None, the session is simply discarded and recreated on next Generate.
        response_tx: oneshot::Sender<Result<(), LlmError>>,
    },
    /// Graceful shutdown of the worker thread.
    Shutdown,
}
```

### LLM Worker Handle

```rust
/// Handle to the LLM worker thread. Owned by the voice assistant service.
/// Cloneable for shared access via `Arc`.
pub struct LlmWorker {
    /// Sender for commands to the worker thread.
    sender: std::sync::mpsc::Sender<LlmWorkerCommand>,
    /// Join handle for clean shutdown.
    handle: Option<std::thread::JoinHandle<()>>,
}
```

---

## 4. Implementation

### 4.1 Worker Thread Loop

The worker thread owns the `LlmInferenceEngine` and an optional `LlmSession`. The session is lazily created on the first `Generate` command and reused for
subsequent commands. When the context window is nearly full, the session is automatically reset.

```rust
fn run_worker(
    engine: LlmInferenceEngine,
    receiver: std::sync::mpsc::Receiver<LlmWorkerCommand>,
) {
    let mut session: Option<LlmSession> = None;
    let mut last_system_prompt: Option<String> = None;

    while let Ok(command) = receiver.recv() {
        match command {
            LlmWorkerCommand::Generate { system_prompt, conversation, max_tokens, response_tx } => {
                let result = handle_generate(
                    &engine,
                    &mut session,
                    &mut last_system_prompt,
                    &system_prompt,
                    &conversation,
                    max_tokens,
                );
                let _ = response_tx.send(result);
            }
            LlmWorkerCommand::Reset { response_tx } => {
                session = None;
                last_system_prompt = None;
                debug!("LLM worker: session reset");
                let _ = response_tx.send(Ok(()));
            }
            LlmWorkerCommand::Shutdown => {
                debug!("LLM worker: shutting down");
                break;
            }
        }
    }
}
```

### 4.2 Generate Handler with Auto-Reset

The `handle_generate` function checks whether the session needs to be (re)created or reset. A reset is triggered in two cases:

1. **No session exists yet** (first call or after explicit reset).
2. **System prompt changed** (tool catalog was updated, invalidating the cached prefix).
3. **Context window nearly full** (estimated token count exceeds `n_ctx * 0.8`).

```rust
fn handle_generate(
    engine: &LlmInferenceEngine,
    session: &mut Option<LlmSession>,
    last_system_prompt: &mut Option<String>,
    system_prompt: &str,
    conversation: &[LlamaChatMessage],
    max_tokens: usize,
) -> Result<(String, Vec<LlamaChatMessage>), LlmError> {
    // Check if system prompt changed (tool catalog update).
    let prompt_changed = last_system_prompt
        .as_ref()
        .is_some_and(|prev| prev != system_prompt);

    // Estimate total token count: ~4 chars per token.
    let conv_chars: usize = conversation.iter().map(|m| m.content.len()).sum();
    let estimated_tokens = (system_prompt.len() + conv_chars) / 4;

    // Check context overflow.
    let n_ctx = engine.config().n_ctx as usize;
    let current_n_cur = session.as_ref().map(|s| s.n_cur as usize).unwrap_or(0);
    let needs_overflow_reset = current_n_cur + estimated_tokens > (n_ctx * 80 / 100);

    // (Re)create session if needed.
    if session.is_none() || prompt_changed || needs_overflow_reset {
        if prompt_changed {
            debug!("LLM worker: resetting session (system prompt changed)");
        } else if needs_overflow_reset {
            debug!("LLM worker: resetting session (context overflow: {} + {} > {})", current_n_cur, estimated_tokens, n_ctx);
        }
        *session = Some(engine.create_session()?);
        *last_system_prompt = Some(system_prompt.to_string());
    }

    // Generate.
    let session = session.as_mut().expect("session should be initialized");
    let output = session.generate(engine.model(), system_prompt, conversation, max_tokens)?;
    Ok((output, conversation.to_vec()))
}
```

### 4.3 LlmWorker Public API

```rust
impl LlmWorker {
    /// Spawns the dedicated LLM worker thread.
    /// Consumes the engine (moved to the worker thread).
    pub fn new(engine: LlmInferenceEngine) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel::<LlmWorkerCommand>();
        let handle = std::thread::Builder::new()
            .name("llm-worker".into())
            .spawn(move || run_worker(engine, receiver))
            .expect("failed to spawn LLM worker thread");
        debug!("LLM worker thread spawned");
        Self {
            sender,
            handle: Some(handle),
        }
    }

    /// Requests a generation from the worker thread.
    /// Returns the LLM output and the updated conversation.
    pub async fn generate(
        &self,
        system_prompt: &str,
        conversation: Vec<LlamaChatMessage>,
        max_tokens: usize,
    ) -> Result<(String, Vec<LlamaChatMessage>), LlmError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(LlmWorkerCommand::Generate {
                system_prompt: system_prompt.to_string(),
                conversation,
                max_tokens,
                response_tx: tx,
            })
            .map_err(|_| LlmError::ChannelClosed)?;
        rx.await
            .map_err(|_| LlmError::ChannelClosed)?
    }

    /// Resets the worker session, discarding the KV cache.
    pub async fn reset(&self) -> Result<(), LlmError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(LlmWorkerCommand::Reset { response_tx: tx })
            .map_err(|_| LlmError::ChannelClosed)?;
        rx.await
            .map_err(|_| LlmError::ChannelClosed)?
    }

    /// Shuts down the worker thread gracefully.
    pub fn shutdown(&mut self) {
        let _ = self.sender.send(LlmWorkerCommand::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for LlmWorker {
    fn drop(&mut self) {
        self.shutdown();
    }
}
```

### 4.4 Error Extension

Add channel-related errors to `LlmError`:

```rust
#[error("Worker channel closed")]
ChannelClosed,
```

---

## 5. Service Integration

### 5.1 Service Field Change

In `VoiceAssistantService` (`service.rs`), replace:

```rust
pub llm_engine: Option<Arc<LlmInferenceEngine> >,
```

with:

```rust
pub llm_worker: Option<Arc<LlmWorker> >,
```

### 5.2 Service Initialization

During `VoiceAssistantService::new`, after loading the model:

```rust
let engine = LlmInferenceEngine::load( & service_config.to_llm_config()) ?;
service.llm_worker = Some(Arc::new(LlmWorker::new(engine)));
```

### 5.3 ReAct Loop Adaptation

In `react.rs`, replace the `spawn_blocking` + `create_session` pattern with a simple async call to the worker:

```rust
pub async fn execute_react_loop(&self, user_text: &str) -> Result<String, AssistantError> {
    let system_prompt = self.build_system_prompt();
    let mut conversation = vec![
        LlamaChatMessage::new("user".to_string(), user_text.to_string())
            .map_err(|error| AssistantError::LlmInference(error.to_string()))?,
    ];

    let worker = self
        .llm_worker
        .as_ref()
        .ok_or(AssistantError::LlmInference("LLM worker not initialized".to_string()))?
        .clone();

    let max_tokens = worker.config().max_tokens;
    let max_iterations = self.config.max_react_iterations;

    for iteration in 0..max_iterations {
        let (llm_output, returned_conversation) = worker
            .generate(&system_prompt, conversation.clone(), max_tokens)
            .await
            .map_err(|error| AssistantError::LlmInference(error.to_string()))?;

        conversation = returned_conversation;

        match parse_llm_response(&llm_output) {
            Ok(LlmResponse::ToolCall { tool, arguments }) => {
                let tool_result = self.invoke_tool(&tool, &arguments).await?;
                conversation.push(
                    LlamaChatMessage::new(
                        "user".to_string(),
                        format!("Tool {tool} result: {tool_result}"),
                    )
                    .map_err(|error| AssistantError::LlmInference(error.to_string()))?,
                );
            }
            Ok(LlmResponse::FinalAnswer { answer }) => {
                return Ok(answer);
            }
            Err(error) => {
                debug!("Voice Assistant: ReAct parse error on iteration {iteration}: {error}");
                if iteration + 1 < max_iterations {
                    conversation.push(
                        LlamaChatMessage::new("assistant".to_string(), llm_output)
                            .map_err(|error| AssistantError::LlmInference(error.to_string()))?,
                    );
                    conversation.push(
                        LlamaChatMessage::new(
                            "user".to_string(),
                            "Your previous response was not valid JSON. Please respond with ONLY a JSON object.".to_string(),
                        )
                        .map_err(|error| AssistantError::LlmInference(error.to_string()))?,
                    );
                } else {
                    return Err(error);
                }
            }
        }
    }

    Err(AssistantError::MaxIterationsReached)
}
```

### 5.4 Tool Catalog Change Detection

When a new tool is registered (`on_tool_registered`), the system prompt changes. The next `build_system_prompt` call will produce a different string. The worker
detects this automatically via the `prompt_changed` check in `handle_generate` and resets the session.

No explicit `reset()` call is needed for tool catalog changes. However, an explicit `reset()` should be called when:

- The user manually requests a fresh session (e.g., via a config change).
- The service is re-enabled after being disabled.

---

## 6. Context Overflow Strategy

### Threshold

The auto-reset threshold is set to **80% of `n_ctx`**:

```rust
let needs_overflow_reset = current_n_cur + estimated_tokens > (n_ctx * 80 / 100);
```

With `n_ctx = 8192`, this triggers at ~6553 tokens. This leaves ~1639 tokens of headroom for generation.

### What Happens on Reset

1. The current `LlmSession` is dropped (KV cache freed).
2. A new `LlamaContext` is created via `model.new_context()`.
3. `n_cur` resets to 0.
4. The full system prompt + conversation is re-processed on the next `generate()` call.
5. The conversation history is preserved (it is passed in from the service, not stored in the session).

### Conversation Trimming (Future Optimization)

For very long conversations, a future enhancement could trim old messages before re-processing:

```rust
// Keep only the last N messages to fit within the context window.
const MAX_CONV_MESSAGES: usize = 10;
if conversation.len() > MAX_CONV_MESSAGES {
conversation = conversation[conversation.len() - MAX_CONV_MESSAGES..].to_vec();
}
```

This is not part of the initial implementation but documented as a follow-up.

---

## 7. Thread Safety Analysis

| Component            | `Send`?                                                | `Sync`? | Owner               |
|----------------------|--------------------------------------------------------|---------|---------------------|
| `LlmInferenceEngine` | Yes (contains `LlamaBackend`, `LlamaModel`)            | Yes     | Worker thread       |
| `LlmSession`         | **No** (`LlamaContext<'a>` is `!Send`)                 | No      | Worker thread       |
| `LlamaBatch`         | No                                                     | No      | Worker thread       |
| `LlamaSampler`       | No                                                     | No      | Worker thread       |
| `LlmWorker` (handle) | Yes (only `mpsc::Sender` + `JoinHandle`)               | Yes     | Service (via `Arc`) |
| `LlmWorkerCommand`   | Yes (contains only `String`, `Vec`, `oneshot::Sender`) | Yes     | Channel             |

The worker thread owns all `!Send` types. The service only holds the `mpsc::Sender` and `JoinHandle`, both of which are `Send + Sync`. This allows the service
to remain async and share the worker handle via `Arc`.

---

## 8. Performance Comparison

### Before (Fresh Session Per Pipeline Run)

```
Command 1: [==== System Prompt Pre-fill ====][== User ==][Gen]  ~1.5s
Command 2: [==== System Prompt Pre-fill ====][== User ==][Gen]  ~1.5s
Command 3: [==== System Prompt Pre-fill ====][== User ==][Gen]  ~1.5s
```

### After (Persistent Session)

```
Command 1: [==== System Prompt Pre-fill ====][== User ==][Gen]  ~1.5s
Command 2:                                       [== User ==][Gen]  ~0.05s
Command 3:                                       [== User ==][Gen]  ~0.05s
Command 4:                                       [== User ==][Gen]  ~0.05s
...
Command N (overflow): [==== Full Re-fill ====][== User ==][Gen]  ~1.5s
```

### Estimated Savings

| Metric                | Before | After   | Improvement |
|-----------------------|--------|---------|-------------|
| 10 commands           | ~15 s  | ~2.0 s  | 7.5x        |
| 50 commands           | ~75 s  | ~4.0 s  | 18.75x      |
| First command latency | ~1.5 s | ~1.5 s  | Same        |
| Steady-state latency  | ~1.5 s | ~0.05 s | 30x         |

---

## 9. Configuration

No new configuration fields are required. The worker uses existing settings:

- `llm_context_size` — determines `n_ctx` and the 80% overflow threshold.
- `llm_threads` — passed to `LlamaContextParams`.
- `llm_temperature`, `llm_top_k`, `llm_top_p` — used in the sampler chain.

The overflow threshold (80%) is a compile-time constant. If configurability is desired later, it can be added to `VoiceAssistantServiceConfig`:

```rust
/// Fraction of n_ctx at which the session auto-resets (0.0-1.0).
pub context_overflow_threshold: f32,  // default: 0.8
```

---

## 10. Testing Strategy

### Unit Tests

- **Worker creation and shutdown**: Verify the thread starts and stops cleanly.
- **Generate with fresh session**: First call produces output.
- **Generate with persistent session**: Second call reuses KV cache (verify `n_cur` increases).
- **System prompt change detection**: Changing the prompt triggers a reset.
- **Context overflow**: Feeding enough tokens triggers an auto-reset.
- **Channel closed handling**: Dropping the response channel does not panic the worker.

### Integration Tests

- **Multi-command pipeline**: Send 5 sequential commands, verify all succeed.
- **Tool invocation between commands**: Verify tool results are correctly fed back.
- **Long conversation**: Send 20+ commands to trigger overflow reset, verify recovery.

### Manual Verification

- Compare `delta_tokens` log output between first and subsequent commands.
- Measure wall-clock latency for the second command vs. the first.

---

## 11. Affected Files

| File                                      | Change                                                                                                                                         |
|-------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------|
| `services/voice_assistant/src/llm.rs`     | Add `LlmWorker`, `LlmWorkerCommand`, `run_worker`, `handle_generate`. Add `ChannelClosed` to `LlmError`.                                       |
| `services/voice_assistant/src/react.rs`   | Replace `spawn_blocking` + `create_session` with `worker.generate()`. Remove `execute_react_loop_with_conversation` (merged into single loop). |
| `services/voice_assistant/src/service.rs` | Replace `llm_engine: Option<Arc<LlmInferenceEngine>>` with `llm_worker: Option<Arc<LlmWorker>>`. Update initialization.                        |

---

## 12. Migration Path

The change is backward-compatible:

1. **Add `LlmWorker`** to `llm.rs` alongside existing types (no removals).
2. **Update `service.rs`** to use `LlmWorker::new(engine)` instead of storing the engine directly.
3. **Update `react.rs`** to call `worker.generate()` instead of `spawn_blocking`.
4. **Remove `execute_react_loop_with_conversation`** — the single `execute_react_loop` method now handles both initial and continuation cases, since the worker
   persists the session.
5. **Build and test**.

No changes to `config.rs`, `model/`, or any other crate are required.
