# Unified Memory Architecture for the Voice Assistant

This document merges three previously developed concepts into a single, coherent memory architecture for the Voice Assistant LLM pipeline. The three source
concepts remain available for detailed reference:

- `concepts/SPEAK_SWIPER_CONTEXT_OVERFLOW_CONCEPT.md` — Persistent LLM Worker Thread & KV-Cache Reuse
- `concepts/SPEAK_SWIPER_SHORT_MEMORY_CONCEPT.md` — Short-Term Memory (Conversation History & Entity Store)
- `concepts/SPEAK_SWIPER_LONG_MEMORY_CONCEPT.md` — Long-Term Memory & Intelligent Context Retrieval (fastembed + nucleo)

---

## 1. Problem Statement

The voice assistant currently suffers from five interconnected limitations:

1. **No KV-cache reuse**: Every `execute_react_loop` call creates a fresh `LlmSession`, re-processing the entire system prompt (~2000 tokens) from scratch.
   Cumulative waste: ~1.5s per command.
2. **No conversation memory**: Each pipeline run starts with an empty conversation. Pronouns like "ihn" in "Mach ihn aus" cannot be resolved.
3. **No device state awareness**: The LLM cannot query whether a device is on or off without making a tool call.
4. **No persistent facts**: After a process restart, all user preferences and learned habits are lost.
5. **Full tool catalog injection**: All 50+ registered MCP tools are serialized into the system prompt (~4000 chars / ~1000 tokens), leaving little room for
   generation and confusing the LLM with irrelevant tools.

### Required Capabilities

| Capability           | Example                                       | Solution                            | Source Concept            |
|----------------------|-----------------------------------------------|-------------------------------------|---------------------------|
| KV-cache persistence | Second command reuses cached prefix           | Dedicated LLM worker thread         | Context Overflow          |
| Pronoun resolution   | "Mach **ihn** aus" → fan                      | Conversation history (RAM, trimmed) | Short-Term Memory         |
| Device state query   | "Läuft der **Ventilator**?"                   | Entity state store (RAM + SQLite)   | Short-Term Memory         |
| Persistent facts     | "Merke: Mein Name ist Alex"                   | SQLite key-value store + fastembed  | Long-Term Memory          |
| Semantic recall      | "Wie soll der Ventilator eingestellt sein?"   | fastembed cosine similarity         | Long-Term Memory          |
| Tool filtering       | 52 tools → top 5 for "Ventilator aus"         | nucleo fuzzy matching               | Long-Term Memory          |
| Entity history       | "Wann habe ich den Ventilator eingeschaltet?" | SQLite entity history               | Long-Term Memory          |
| Habit learning       | "User schaltet Ventilator immer um 22:00 aus" | Pattern analysis on entity history  | Long-Term Memory (future) |

---

## 2. Architecture Overview

```
+------------------------------------------------------------------------------------------+
| Voice Assistant Service (async)                                                          |
|                                                                                          |
|  +------------------+   +-------------------+   +-------------------+   +-------------+  |
|  | Conversation     |   | Entity Store      |   | Semantic Memory   |   | Tool Router |  |
|  | History          |   | (RAM + SQLite)    |   | (fastembed + DB)  |   | (nucleo)    |  |
|  | Arc<RwLock<Vec>> |   | Arc<RwLock<HM>>   |   | RwLock<SemMem>    |   | ToolRouter  |  |
|  +------------------+   +-------------------+   +-------------------+   +-------------+  |
|         |                       |                       |                      |          |
|         v                       v                       v                      v          |
|  +---------------------------------------------------------------------------+           |
|  | build_system_prompt(user_text)                                            |           |
|  |   1. nucleo: select top 5 tools for user_text                             |           |
|  |   2. Entity summary from entity store                                     |           |
|  |   3. fastembed: semantic recall of top 3 facts for user_text              |           |
|  |   4. Replace {tools}, {entities}, {long_term} in template                 |           |
|  +---------------------------------------------------------------------------+           |
|         |                                                                                |
|         v                                                                                |
|  +------------------+     mpsc channel      +------------------------------------------+  |
|  | LlmWorker Handle |--------------------->| LLM Worker Thread (dedicated OS thread) |  |
|  | Arc<LlmWorker>   |<----- oneshot -------|                                          |  |
|  +------------------+                      | Holds: LlmSession (KV-cache, !Send)     |  |
|                                            | Auto-reset on: context overflow (80%)   |  |
|                                            | System prompt stable across commands     |  |
|                                            +------------------------------------------+  |
|                                                                                          |
|  +-------------------+                                                                    |
|  | SQLite DB         |  ~/.local/share/smearor/memory.db                                 |
|  | - facts           |  Persistent across restarts                                       |
|  | - entity_history  |  Write-through from entity store                                  |
|  +-------------------+                                                                    |
+------------------------------------------------------------------------------------------+
```

### Four Memory Layers

| Layer                        | Scope                      | Lifetime                    | Storage                    | Latency      |
|------------------------------|----------------------------|-----------------------------|----------------------------|--------------|
| **L0: KV-Cache**             | LLM context prefix         | Session (reset on overflow) | RAM (worker thread)        | 0 ms (reuse) |
| **L1: Conversation History** | Dialog messages            | Session (volatile)          | RAM (`Arc<RwLock<Vec>>`)   | < 1 ms       |
| **L2: Entity Store**         | Device states              | Process lifetime + SQLite   | RAM + SQLite               | < 1 ms       |
| **L3: Semantic Memory**      | Facts, preferences, habits | Permanent                   | SQLite + fastembed vectors | ~10 ms       |

---

## 3. Layer L0: Persistent LLM Worker (KV-Cache Reuse)

### Problem

`LlmSession` holds a `LlamaContext` which is `!Send`. The current architecture uses `tokio::task::spawn_blocking`, which does not guarantee the same thread
across calls. Therefore, the session cannot be persisted.

### Solution: Dedicated Worker Thread

A dedicated OS thread owns all `!Send` types and lives for the entire service lifetime. Communication via `std::sync::mpsc` (commands) and
`tokio::sync::oneshot` (responses).

```rust
/// Commands sent from the async service to the LLM worker thread.
enum LlmWorkerCommand {
    /// Generate a completion from the system prompt and conversation.
    Generate {
        system_prompt: String,
        conversation: Vec<LlamaChatMessage>,
        max_tokens: usize,
        response_tx: oneshot::Sender<Result<(String, Vec<LlamaChatMessage>), LlmError>>,
    },
    /// Reset the session (discard KV cache).
    Reset {
        response_tx: oneshot::Sender<Result<(), LlmError>>,
    },
    /// Graceful shutdown.
    Shutdown,
}

/// Handle to the LLM worker thread. Owned by the voice assistant service.
pub struct LlmWorker {
    sender: std::sync::mpsc::Sender<LlmWorkerCommand>,
    handle: Option<std::thread::JoinHandle<()>>,
}
```

### Auto-Reset Triggers

The worker resets the session (discards KV cache, creates new `LlamaContext`) in three cases:

1. **No session exists** (first call or after explicit reset).
2. **System prompt changed** (only when the stable system prompt template text itself changes, not when dynamic content like tools/entities/facts changes — see
   Section 7).
3. **Context window nearly full** (`n_cur + exact_token_count > n_ctx * 0.8`).

```rust
fn handle_generate(
    engine: &LlmInferenceEngine,
    session: &mut Option<LlmSession>,
    last_system_prompt: &mut Option<String>,
    system_prompt: &str,
    conversation: &[LlamaChatMessage],
    max_tokens: usize,
) -> Result<(String, Vec<LlamaChatMessage>), LlmError> {
    let prompt_changed = last_system_prompt
        .as_ref()
        .is_some_and(|prev| prev != system_prompt);

    // Exact token count via the model's native tokenizer.
    // The chars/4 heuristic is inaccurate for German text (Umlaute, compound words)
    // and JSON syntax, which inflate the real token count to ~chars/2.5-3.
    // Using str_to_token ensures the overflow threshold is precise.
    let n_ctx = engine.config().n_ctx as usize;
    let current_n_cur = session.as_ref().map(|s| s.n_cur as usize).unwrap_or(0);

    // Build the full formatted prompt to get the exact token count.
    // This duplicates the tokenization in generate(), but the cost is negligible
    // (string lookup, no neural network) compared to the decode cost.
    let mut all_messages = vec![
        LlamaChatMessage::new("system".to_string(), system_prompt.to_string())
            .map_err(|e| LlmError::ChatMessage(e.to_string()))?,
    ];
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

    let needs_overflow_reset = current_n_cur + exact_token_count > (n_ctx * 80 / 100);

    if session.is_none() || prompt_changed || needs_overflow_reset {
        if prompt_changed {
            debug!("LLM worker: resetting session (system prompt changed)");
        } else if needs_overflow_reset {
            debug!("LLM worker: resetting session (context overflow: {} + {} > {})", current_n_cur, exact_token_count, n_ctx);
        }
        *session = Some(engine.create_session()?);
        *last_system_prompt = Some(system_prompt.to_string());
    }

    let session = session.as_mut().expect("session should be initialized");
    let output = session.generate(engine.model(), system_prompt, conversation, max_tokens)?;
    Ok((output, conversation.to_vec()))
}
```

### Performance Impact

| Scenario              | Before (fresh session)           | After (persistent)      |
|-----------------------|----------------------------------|-------------------------|
| First command         | ~1.5s (full pre-fill)            | ~1.5s (full pre-fill)   |
| Second command        | ~1.5s (re-process system prompt) | ~0.05s (delta only)     |
| 10th command          | ~1.5s                            | ~0.05s                  |
| Overflow reset        | —                                | ~1.5s (full re-fill)    |
| **10 commands total** | **~15s**                         | **~2.0s** (7.5x faster) |

### Thread Safety

| Component            | `Send`? | `Sync`?          | Owner               |
|----------------------|---------|------------------|---------------------|
| `LlmInferenceEngine` | Yes     | **Not required** | Worker thread       |
| `LlmSession`         | **No**  | No               | Worker thread       |
| `LlmWorker` (handle) | Yes     | Yes              | Service (via `Arc`) |
| `LlmWorkerCommand`   | Yes     | Yes              | Channel             |

`LlmInferenceEngine` lives exclusively inside the worker thread and is never shared across threads. It only needs `Send` (to be moved into the thread at spawn
time), not `Sync`. This is advantageous because some low-level llama.cpp bindings may not implement `Sync` reliably. The only component that must be
`Send + Sync` on the async Tokio side is the `LlmWorker` handle, which communicates via channels and holds no `!Send` types.

---

## 4. Layer L1: Conversation History (Short-Term)

### Data Structure

```rust
/// Recent conversation messages retained across pipeline runs.
/// Trimmed to a maximum number of messages to prevent context overflow.
pub type ConversationHistory = Arc<RwLock<Vec<LlamaChatMessage>>>;
```

### ReAct Loop Integration

```rust
pub async fn execute_react_loop(&self, user_text: &str) -> Result<String, AssistantError> {
    let system_prompt = self.build_system_prompt(user_text);

    // Load history and append new user message.
    let mut conversation = self
        .conversation_history
        .read()
        .map(|h| h.clone())
        .unwrap_or_default();

    conversation.push(
        LlamaChatMessage::new("user".to_string(), user_text.to_string())
            .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
    );

    // ... run ReAct loop via worker.generate() ...

    // After final answer: append assistant message and save.
    conversation.push(
        LlamaChatMessage::new("assistant".to_string(), final_answer.clone())
            .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
    );

    // Trim to last N messages.
    let max_messages = self.config.max_history_messages;
    if conversation.len() > max_messages {
        let start = conversation.len() - max_messages;
        conversation = conversation.split_off(start);
    }

    if let Ok(mut history) = self.conversation_history.write() {
        *history = conversation;
    }

    Ok(final_answer)
}
```

### Trimming Strategy

With `max_history_messages = 10` and ~50 tokens per message: ~500 tokens (~6% of `n_ctx = 8192`). Only user and assistant messages are stored. Tool results are
transient within a single ReAct loop.

### Interaction with L0 (KV-Cache)

The conversation history is passed to `worker.generate()` on every call. The worker's delta-only decode processes only new messages. On auto-reset (context
overflow), the full trimmed history is re-processed from scratch.

---

## 5. Layer L2: Entity State Store (Short-Term + SQLite Write-Through)

### Data Model

```rust
/// Represents the state of a controllable entity (e.g., a smart home device).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntityState {
    /// Human-readable name of the entity (e.g., "Ventilator").
    pub name: String,
    /// Current state (e.g., "on", "off", "open", "closed").
    pub state: String,
    /// Tool name that controls this entity (e.g., "button_shelly_fan_button").
    pub tool: String,
    /// Last action performed (e.g., "click", "longpress").
    pub last_action: String,
    /// ISO 8601 timestamp of the last state change.
    pub last_changed: String,
}

/// In-memory store of entity states, keyed by entity identifier.
pub type EntityStore = Arc<RwLock<HashMap<String, EntityState>>>;
```

### Automatic State Extraction

After each successful tool invocation, the service parses the tool name and arguments:

```rust
/// Extracts entity state from a tool call and its result.
fn extract_entity_state(tool_name: &str, arguments: &serde_json::Value) -> Option<EntityState> {
    // Button tools: "button_<plugin_id>"
    if let Some(plugin_id) = tool_name.strip_prefix("button_") {
        let action = arguments.get("action").and_then(|v| v.as_str()).unwrap_or("click");
        let state = match action {
            "click" => "on",
            "longpress" => "off",
            _ => return None,
        };
        return Some(EntityState {
            name: plugin_id.replace('_', " "),
            state: state.to_string(),
            tool: tool_name.to_string(),
            last_action: action.to_string(),
            last_changed: chrono::Utc::now().to_rfc3339(),
        });
    }

    // App launcher tools
    if tool_name == "app_launcher_exec" || tool_name == "app_launcher_terminate" {
        // ... similar extraction ...
    }

    None
}
```

### Write-Through to SQLite

When the short-term entity store updates, the change is also written to the SQLite entity history:

```rust
fn update_entity_state(&self, tool_name: &str, arguments: &serde_json::Value) {
    if let Some(state) = Self::extract_entity_state(tool_name, arguments) {
        // Update short-term store (RAM).
        if let Ok(mut store) = self.entity_store.write() {
            store.insert(tool_name.to_string(), state.clone());
        }

        // Write-through to long-term store (SQLite).
        if let Ok(db) = self.entity_db.lock() {
            let _ = db.execute(
                "INSERT INTO entity_history (entity, state, action, tool, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![state.name, state.state, state.last_action, state.tool, state.last_changed],
            );
        }
    }
}
```

### Reconstruction on Startup

When the service starts, the short-term entity store is reconstructed from the most recent entity history entries in SQLite. Only the latest state per entity is
kept.

### MCP Integration

- **Resource** `memory://entities`: Returns all entity states as JSON.
- **Tool** `memory_query`: Queries a specific entity by name or tool name.

### Entity State Inference Rules

| Tool Pattern                 | Action       | Inferred State       |
|------------------------------|--------------|----------------------|
| `button_*`                   | `click`      | `on`                 |
| `button_*`                   | `longpress`  | `off`                |
| `button_*`                   | `swipe_up`   | `increasing`         |
| `button_*`                   | `swipe_down` | `decreasing`         |
| `app_launcher_exec`          | —            | `running`            |
| `app_launcher_terminate`     | —            | `stopped`            |
| `audio_set_volume`           | —            | volume value         |
| `mpris_play` / `mpris_pause` | —            | `playing` / `paused` |

---

## 6. Layer L3: Long-Term Memory & Intelligent Context Retrieval

### 6.1 nucleo: Intelligent Tool Router

Tool names and descriptions are short, keyword-dense strings. Fuzzy matching is ideal because:

- **Speed**: nucleo matches 50+ tools in **microseconds** (Smith-Waterman algorithm with aggressive prefiltering).
- **Keyword precision**: "Ventilator aus" directly matches tool name `button_shelly_fan_ventilator`.
- **No model loading**: Pure algorithm, no ONNX runtime, no RAM overhead.
- **Multilingual**: Handles Unicode graphemes correctly, matching German tool descriptions.

```rust
use nucleo_matcher::{Config, Matcher, Pattern, CaseMatching};

/// Tool router that fuzzy-matches user text against the tool catalog.
/// Rebuilt when tools are registered/unregistered.
pub struct ToolRouter {
    config: Config,
    tools: Vec<ToolEntry>,
}

struct ToolEntry {
    name: String,
    description: String,
    input_schema: String,
    /// Combined matchable text: "name description" for nucleo.
    match_text: String,
}

impl ToolRouter {
    /// Creates a new tool router with default configuration.
    pub fn new() -> Self {
        Self {
            config: Config::DEFAULT,
            tools: Vec::new(),
        }
    }

    /// Rebuilds the router from the current tool catalog.
    pub fn rebuild(&mut self, catalog: &[ToolCatalogEntry]) {
        self.tools = catalog
            .iter()
            .map(|t| ToolEntry {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
                match_text: format!("{} {}", t.name, t.description),
            })
            .collect();
        debug!("Tool router: rebuilt with {} tools", self.tools.len());
    }

    /// Returns the top N tools that fuzzy-match the user's query.
    pub fn select_tools(&self, user_text: &str, top_n: usize) -> Vec<&ToolEntry> {
        let mut matcher = Matcher::new(self.config);
        let pattern = Pattern::parse(user_text, CaseMatching::Smart);

        let mut scored: Vec<(f64, &ToolEntry)> = self
            .tools
            .iter()
            .filter_map(|tool| {
                let mut haystack = nucleo_matcher::Utf32String::from(&tool.match_text);
                let score = pattern.score(&mut matcher, &mut haystack);
                if score > 0 {
                    Some((score as f64, tool))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored.into_iter().take(top_n).map(|(_, tool)| tool).collect()
    }
}
```

**Fallback**: If nucleo returns zero matches (e.g., "Hallo" with no tool intent), a minimal set of always-available tools is injected:

```rust
const ALWAYS_AVAILABLE_TOOLS: &[&str] = &[
    "voice_assistant_activate",
    "voice_assistant_deactivate",
    "voice_assistant_submit_text",
    "memory_store",
    "memory_recall",
    "memory_list",
    "memory_forget",
];
```

**Performance Impact**:

| Metric                    | Before (all tools)         | After (nucleo top 5)     |
|---------------------------|----------------------------|--------------------------|
| Tool catalog in prompt    | ~4000 chars / ~1000 tokens | ~400 chars / ~100 tokens |
| Tool selection time       | 0 ms                       | < 1 ms                   |
| Context saved per command | —                          | ~900 tokens              |

### 6.2 fastembed: Semantic Vector Memory

Facts are natural language sentences with semantic meaning. Keyword matching fails when the user stores "Ich arbeite meistens im Home-Office" and later asks "Wo
ist mein Büro?" — the semantic connection is missed.

**Recommended model**: `BAAI/bge-small-en-v1.5` (quantized `BGESmallENV15Q`), ~80 MB RAM, INT8 quantization, 384 dimensions. For German support:
`paraphrase-multilingual-MiniLM-L12-v2` (~470 MB).

```rust
use fastembed::{TextEmbedding, TextInitOptions, EmbeddingModel};

/// Semantic memory store using fastembed for embedding generation.
/// Embeddings are stored in-memory and persisted to SQLite.
pub struct SemanticMemory {
    /// The fastembed text embedding model.
    model: TextEmbedding,
    /// In-memory vector store: (embedding, fact_id).
    vectors: Vec<(Vec<f32>, String)>,
    /// SQLite connection for persistence.
    db: rusqlite::Connection,
}

/// A stored fact with its embedding.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredFact {
    pub id: String,
    pub key: String,
    pub value: String,
    pub category: FactCategory,
    pub embedding: Vec<f32>,
    pub created_at: String,
    pub last_accessed: String,
    pub access_count: u32,
}

/// Category of a stored fact.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FactCategory {
    /// A simple factual statement.
    Fact,
    /// A user preference.
    Preference,
    /// A learned or explicitly stored habit.
    Habit,
}
```

**Core operations**:

```rust
impl SemanticMemory {
    /// Initializes the semantic memory store with the embedding model.
    pub fn new(db_path: &str) -> Result<Self, MemoryError> {
        let model = TextEmbedding::try_new(
            TextInitOptions::new(EmbeddingModel::BGESmallENV15Q)
                .with_intra_threads(2),
        )?;
        let db = rusqlite::Connection::open(db_path)?;
        Self::init_schema(&db)?;
        let mut memory = Self { model, vectors: Vec::new(), db };
        memory.load_vectors_from_db()?;
        Ok(memory)
    }

    /// Stores a fact with its semantic embedding.
    pub fn store(&mut self, key: &str, value: &str, category: FactCategory) -> Result<String, MemoryError> {
        let id = uuid::Uuid::new_v4().to_string();
        let embedding = self.model.embed(vec![value.to_string()], None)?
            .into_iter().next().ok_or(MemoryError::EmbeddingFailed)?;
        let fact = StoredFact {
            id: id.clone(), key: key.to_string(), value: value.to_string(),
            category, embedding: embedding.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_accessed: chrono::Utc::now().to_rfc3339(),
            access_count: 0,
        };
        self.persist_fact(&fact)?;
        self.vectors.push((embedding, id.clone()));
        debug!("Semantic memory: stored fact '{key}' with id {id}");
        Ok(id)
    }

    /// Recalls facts semantically related to the query.
    /// Returns the top N facts by cosine similarity.
    pub fn recall(&mut self, query: &str, top_n: usize) -> Result<Vec<StoredFact>, MemoryError> {
        if self.vectors.is_empty() { return Ok(Vec::new()); }
        let query_embedding = self.model.embed(vec![query.to_string()], None)?
            .into_iter().next().ok_or(MemoryError::EmbeddingFailed)?;
        let mut scored: Vec<(f32, &str)> = self.vectors.iter()
            .map(|(emb, id)| (cosine_similarity(&query_embedding, emb), id.as_str()))
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let top_ids: Vec<&str> = scored.iter().take(top_n).map(|(_, id)| *id).collect();
        let facts = self.load_facts_by_ids(&top_ids)?;
        for fact in &facts { self.touch_fact(&fact.id)?; }
        Ok(facts)
    }
}
```

**Performance characteristics**:

| Operation                  | Time     | Notes                            |
|----------------------------|----------|----------------------------------|
| Model load (startup)       | ~500 ms  | One-time, cached after first run |
| Embed single query         | ~5-10 ms | ONNX forward pass on CPU         |
| Store fact + embedding     | ~10 ms   | Embed + SQLite write             |
| Recall (100 facts, top 3)  | ~10 ms   | Embed query + cosine similarity  |
| Recall (1000 facts, top 3) | ~15 ms   | Linear scan                      |
| RAM (model + 1000 facts)   | ~170 MB  | ~80 MB model + ~90 MB vectors    |

**Scalability note**: The flat `Vec<(Vec<f32>, String)>` with linear `O(N)` scan is ideal for the first few hundred facts — negligible CPU cost, zero indexing
overhead. If the knowledge base grows to thousands of entries over time and latency becomes measurable, replace the flat vector with an in-memory ANN index
crate such as `hnsw_rs` or `space` to reduce search complexity from `O(N)` to `O(log N)`. The `SemanticMemory` struct's `vectors` field is the only type that
needs to change; the `recall` API stays identical. This is a future optimization and not required for the initial implementation.

### 6.3 MCP Tools for Long-Term Memory

Four MCP tools are registered for LLM-driven memory operations:

- **`memory_store`** — Store a fact or preference with semantic embedding.
- **`memory_recall`** — Recall facts using semantic search (natural language query).
- **`memory_list`** — List all stored fact keys, optionally filtered by category.
- **`memory_forget`** — Delete a fact by key.

---

## 7. System Prompt Architecture

### Critical Design Decision: Stable System Prompt + Dynamic Context Message

The system prompt must remain **stable across commands** to maximize KV-cache reuse (L0). If `{tools}` were in the system prompt, nucleo would select different
tools per query, changing the prompt every command and triggering a worker reset every time.

**Solution**: Split the prompt into two parts:

1. **System prompt** (stable, rarely changes): Role description, response format instructions. No dynamic content.
2. **Context user message** (changes per command): Selected tools, entity states, recalled facts.

```rust
/// Builds the stable system prompt (no dynamic content).
/// This prompt is cached in the KV-cache and reused across commands.
pub fn build_system_prompt(&self) -> String {
    let template = self.config.system_prompt.as_deref().unwrap_or(DEFAULT_PROMPT);
    template.to_string()
}

/// Builds the dynamic context message injected as the first user message.
/// This changes per command but does NOT trigger a worker reset.
pub fn build_context_message(&self, user_text: &str) -> String {
    // 1. nucleo: select top 5 relevant tools.
    let selected_tools = self.tool_router.select_tools(user_text, self.config.max_tools_in_prompt);
    let tools_json = self.serialize_tools(&selected_tools);

    // 2. Entity states (from short-term memory).
    let entity_summary = self.build_entity_summary();

    // 3. fastembed: semantic recall of top 3 relevant facts.
    let long_term_summary = self.build_long_term_summary(user_text);

    format!(
        "Available tools: {tools_json}.\nKnown device states:\n{entity_summary}\nKnown facts:\n{long_term_summary}"
    )
}
```

### Prompt Template

```toml
[voice_assistant]
system_prompt = "Du bist ein Smart-Home-Assistent. Antworte in JSON. Tool-Aufruf: {\"tool\": \"<name>\", \"arguments\": {...}}. Finale Antwort: {\"final_answer\": \"<text>\"}. Sei präzise und effizient."
```

### Worker Reset Triggers (Revised)

| Trigger                                              | Resets KV-Cache?                      | Frequency             |
|------------------------------------------------------|---------------------------------------|-----------------------|
| System prompt template text changes                  | Yes                                   | Rare (config change)  |
| Tool catalog changes (tools registered/unregistered) | **No** (tools are in context message) | Moderate              |
| Entity states change                                 | **No** (in context message)           | Frequent              |
| Long-term facts change                               | **No** (in context message)           | Rare                  |
| Context overflow (80% of `n_ctx`)                    | Yes                                   | ~every 30-50 commands |

This design ensures the KV-cache is reused maximally. The worker only resets on overflow or explicit config changes.

### Token Budget

| Source                                             | Tokens   |
|----------------------------------------------------|----------|
| System prompt (stable)                             | ~50      |
| Context message: tools (nucleo top 5)              | ~100     |
| Context message: entity states                     | ~100     |
| Context message: long-term facts (fastembed top 3) | ~50      |
| Conversation history (10 msgs)                     | ~500     |
| **Total memory overhead**                          | **~800** |
| Free for generation                                | ~7392    |

---

## 8. SQLite Schema

```sql
CREATE TABLE IF NOT EXISTS facts (
    id TEXT PRIMARY KEY,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'fact',
    embedding BLOB NOT NULL,
    created_at TEXT NOT NULL,
    last_accessed TEXT NOT NULL,
    access_count INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS entity_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity TEXT NOT NULL,
    state TEXT NOT NULL,
    action TEXT NOT NULL,
    tool TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_facts_key ON facts(key);
CREATE INDEX IF NOT EXISTS idx_facts_category ON facts(category);
CREATE INDEX IF NOT EXISTS idx_entity_history_entity ON entity_history(entity);
CREATE INDEX IF NOT EXISTS idx_entity_history_timestamp ON entity_history(timestamp);
```

### Embedding Serialization

Embeddings are stored as BLOBs (little-endian f32 bytes) to avoid JSON overhead:

```rust
fn serialize_embedding(emb: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(emb.len() * 4);
    for &v in emb {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    bytes
}

fn deserialize_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}
```

---

## 9. Combined Pipeline Flow

### Example: Multi-Turn with Memory

```
User says: "Mach den Ventilator aus, aber merk dir dass ich ihn abends immer ausschalte"

1. STT → "Mach den Ventilator aus, aber merk dir dass ich ihn abends immer ausschalte"
2. nucleo: select top 5 tools for "Ventilator aus" → [button_shelly_fan_ventilator, ...]
3. fastembed: recall top 3 facts for "Ventilator abends ausschalten" → [fact: "fan_preference: level 2 in summer"]
4. build_system_prompt(): "Du bist ein Smart-Home-Assistent..." (stable, cached in KV-cache)
5. build_context_message(): "Tools: [...]. Entities: Ventilator: on. Facts: fan_preference: level 2 in summer."
6. LLM ReAct iteration 1:
   - Tool call: button_shelly_fan_ventilator(action=longpress)
   - Entity store updated: Ventilator → off (RAM + SQLite write-through)
7. LLM ReAct iteration 2:
   - Tool call: memory_store(key="fan_evening_off", value="User schaltet den Ventilator abends immer aus", category="habit")
   - Fact stored in SQLite with fastembed embedding
8. LLM ReAct iteration 3:
   - Final answer: "Ventilator ausgeschaltet. Ich habe mir gemerkt, dass du ihn abends immer ausschaltest."
9. Conversation history updated: [user: "Mach...", assistant: "Ventilator ausgeschaltet..."]

Next command: "Mach ihn wieder an"
1. STT → "Mach ihn wieder an"
2. nucleo: select top 5 tools → [button_shelly_fan_ventilator, ...]
3. fastembed: recall → [fact: "fan_preference: level 2 in summer", habit: "fan_evening_off"]
4. System prompt: same as before (KV-cache reused, ~0.05s)
5. Context message: "Tools: [...]. Entities: Ventilator: off. Facts: ..."
6. Conversation history loaded: [..., user: "Mach den Ventilator aus...", assistant: "Ventilator ausgeschaltet..."]
7. LLM resolves "ihn" → Ventilator (from history + entity states)
8. Tool call: button_shelly_fan_ventilator(action=click)
9. Entity store updated: Ventilator → on
10. Final answer: "Ventilator eingeschaltet."
```

### Example: After Process Restart

```
Service starts:
1. SQLite opened: ~/.local/share/smearor/memory.db
2. Entity store reconstructed from entity_history (latest state per entity)
3. fastembed model loaded (~500ms), vectors loaded from SQLite
4. LLM worker thread spawned, fresh session

User says: "Ist der Ventilator an?"
1. Entity store has: Ventilator: off (from SQLite reconstruction)
2. Context message: "Entities: Ventilator: off"
3. LLM answers directly: "Nein, der Ventilator ist aus." (no tool call needed)
```

---

## 10. Service Integration

### VoiceAssistantService Fields

```rust
pub struct VoiceAssistantService {
    // L0: Persistent LLM Worker
    pub llm_worker: Option<Arc<LlmWorker>>,

    // L1: Conversation History
    pub conversation_history: ConversationHistory,

    // L2: Entity Store
    pub entity_store: EntityStore,
    pub entity_db: Arc<Mutex<rusqlite::Connection>>,

    // L3: Long-Term Memory
    pub semantic_memory: Arc<RwLock<SemanticMemory>>,
    pub tool_router: Arc<RwLock<ToolRouter>>,

    // Existing fields
    pub tool_catalog: Arc<RwLock<Vec<ToolCatalogEntry>>>,
    pub config: VoiceAssistantServiceConfig,
    // ...
}
```

### ReAct Loop (Unified)

The context message (tools, entities, facts) is **transient** — it must never be persisted into `conversation_history`. Otherwise stale tool lists and outdated
device states would accumulate in the short-term memory across commands. Two separate vectors are used: `conversation` for the persistent history (only real
user and assistant messages) and `active_payload` for the transient LLM payload (context message + history + tool results, discarded after the loop).

```rust
pub async fn execute_react_loop(&self, user_text: &str) -> Result<String, AssistantError> {
    let system_prompt = self.build_system_prompt();

    // 1. Load the PURE history (only real user and assistant messages).
    let mut conversation = self.conversation_history.read().map(|h| h.clone()).unwrap_or_default();

    // 2. Build the transient context message for this command.
    let context_message = self.build_context_message(user_text);

    // 3. Create a separate vector for the current LLM call.
    let mut active_payload = Vec::with_capacity(conversation.len() + 2);
    active_payload.push(LlamaChatMessage::new("user".to_string(), context_message)?);
    active_payload.extend(conversation.clone());
    active_payload.push(LlamaChatMessage::new("user".to_string(), user_text.to_string())?);

    // 4. Add the user message to the persistent conversation (saved after the loop).
    conversation.push(LlamaChatMessage::new("user".to_string(), user_text.to_string())?);

    let worker = self.llm_worker.as_ref().ok_or(AssistantError::LlmInference("LLM worker not initialized".to_string()))?.clone();
    let max_tokens = worker.config().max_tokens;
    let max_iterations = self.config.max_react_iterations;

    for iteration in 0..max_iterations {
        let (llm_output, _) = worker
            .generate(&system_prompt, active_payload.clone(), max_tokens)
            .await
            .map_err(|error| AssistantError::LlmInference(error.to_string()))?;

        match parse_llm_response(&llm_output) {
            Ok(LlmResponse::ToolCall { tool, arguments }) => {
                let tool_result = self.invoke_tool(&tool, &arguments).await?;
                self.update_entity_state(&tool, &arguments);
                // Tool results go into active_payload ONLY (transient, never persisted).
                active_payload.push(LlamaChatMessage::new("user".to_string(), format!("Tool {tool} result: {tool_result}"))?);
            }
            Ok(LlmResponse::FinalAnswer { answer }) => {
                // Only the final answer is persisted — not the context message or tool results.
                conversation.push(LlamaChatMessage::new("assistant".to_string(), answer.clone())?);
                self.save_conversation_history(conversation);
                return Ok(answer);
            }
            Err(error) => {
                debug!("Voice Assistant: ReAct parse error on iteration {iteration}: {error}");
                if iteration + 1 < max_iterations {
                    active_payload.push(LlamaChatMessage::new("assistant".to_string(), llm_output)?);
                    active_payload.push(LlamaChatMessage::new("user".to_string(), "Please respond with valid JSON.".to_string())?);
                } else {
                    self.save_conversation_history(conversation);
                    return Err(error);
                }
            }
        }
    }

    self.save_conversation_history(conversation);
    Err(AssistantError::MaxIterationsReached)
}
```

---

## 11. Configuration

All new fields in `VoiceAssistantServiceConfig`:

```rust
/// Maximum number of conversation messages to retain in short-term memory.
pub max_history_messages: usize,           // default: 10

/// Whether to inject entity states into the context message.
pub inject_entity_states: bool,            // default: true

/// Path to the SQLite database file for long-term memory.
pub memory_db_path: String,                // default: "~/.local/share/smearor/memory.db"

/// Maximum number of tools to inject into the context message after nucleo filtering.
pub max_tools_in_prompt: usize,            // default: 5

/// Whether to inject semantically recalled facts into the context message.
pub inject_long_term_facts: bool,          // default: true

/// Number of facts to recall via fastembed for context message injection.
pub max_recalled_facts: usize,             // default: 3

/// Embedding model to use for semantic memory.
pub embedding_model: String,               // default: "bge-small-en-v1.5-q"

/// Fraction of n_ctx at which the session auto-resets.
pub context_overflow_threshold: f32,       // default: 0.8
```

**TOML usage**:

```toml
[voice_assistant]
max_history_messages = 10
inject_entity_states = true
memory_db_path = "~/.local/share/smearor/memory.db"
max_tools_in_prompt = 5
inject_long_term_facts = true
max_recalled_facts = 3
embedding_model = "bge-small-en-v1.5-q"
context_overflow_threshold = 0.8
system_prompt = "Du bist ein Smart-Home-Assistent. Antworte in JSON. Tool-Aufruf: {\"tool\": \"<name>\", \"arguments\": {...}}. Finale Antwort: {\"final_answer\": \"<text>\"}."
```

---

## 12. Dependencies

### New Crate Dependencies

| Crate            | Version | Purpose                     | RAM Impact                   |
|------------------|---------|-----------------------------|------------------------------|
| `nucleo-matcher` | 0.3     | Fuzzy tool routing (L3)     | ~0 MB (pure algorithm)       |
| `fastembed`      | 5.17    | Semantic embeddings (L3)    | ~80-150 MB (model dependent) |
| `rusqlite`       | 0.31    | SQLite persistence (L2, L3) | ~2 MB                        |
| `chrono`         | 0.4     | Timestamps (L2, L3)         | ~0 MB (already used)         |
| `uuid`           | 1.0     | Fact IDs (L3)               | ~0 MB (already used)         |

### No New Model Crate

All memory types are internal to the voice assistant service. No FFI sharing is required. If other services need to query entity states in the future, the types
can be moved to `model/voice_assistant/` with `#[stabby::stabby]` annotations.

---

## 13. Affected Files

| File                                           | Change                                                                                                                                                                                                            |
|------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `services/voice_assistant/src/llm.rs`          | Add `LlmWorker`, `LlmWorkerCommand`, `run_worker`, `handle_generate`. Add `ChannelClosed` to `LlmError`.                                                                                                          |
| `services/voice_assistant/src/tool_router.rs`  | **New file**: `ToolRouter` with nucleo matching.                                                                                                                                                                  |
| `services/voice_assistant/src/memory.rs`       | **New file**: `SemanticMemory` with fastembed + SQLite. `EntityState`, `StoredFact`, `FactCategory`.                                                                                                              |
| `services/voice_assistant/src/tool_catalog.rs` | Split `build_system_prompt` into stable prompt + `build_context_message`. Use `ToolRouter`. Inject `{entities}` and `{long_term}`.                                                                                |
| `services/voice_assistant/src/mcp.rs`          | Register `memory://entities` resource, `memory_query` tool, `memory_store`/`memory_recall`/`memory_list`/`memory_forget` tools. Add handlers.                                                                     |
| `services/voice_assistant/src/react.rs`        | Replace `spawn_blocking` with `worker.generate()`. Load/save conversation history. Extract entity state after tool calls. Write entity history to SQLite.                                                         |
| `services/voice_assistant/src/service.rs`      | Replace `llm_engine` with `llm_worker`. Add `conversation_history`, `entity_store`, `entity_db`, `semantic_memory`, `tool_router` fields. Initialize in `new()`. Reconstruct entity store from SQLite on startup. |
| `services/voice_assistant/src/config.rs`       | Add all new config fields.                                                                                                                                                                                        |
| `services/voice_assistant/Cargo.toml`          | Add `nucleo-matcher`, `fastembed`, `rusqlite`, `chrono`, `uuid` dependencies.                                                                                                                                     |

---

## 14. Testing Strategy

### Unit Tests

- **L0 Worker creation and shutdown**: Verify thread starts and stops cleanly.
- **L0 Generate with persistent session**: Second call reuses KV cache (verify `n_cur` increases).
- **L0 System prompt change detection**: Changing the stable prompt triggers a reset.
- **L0 Context overflow**: Feeding enough tokens triggers an auto-reset.
- **L1 Conversation history trimming**: Verify only last N messages are retained.
- **L2 Entity state extraction**: Test `extract_entity_state` for button, app launcher, and unknown tools.
- **L2 Entity state query**: Test `memory_query` tool handler with exact and fuzzy match.
- **L2 Entity resource**: Test `memory://entities` returns correct JSON.
- **L2 Entity reconstruction**: Seed SQLite with history, reconstruct store, verify latest states.
- **L3 nucleo tool router**: Test `select_tools` with various queries. Verify top 5 results are relevant.
- **L3 nucleo empty match**: Verify fallback to always-available tools.
- **L3 fastembed embedding**: Verify embeddings are generated and have correct dimensions.
- **L3 Cosine similarity**: Test with known similar and dissimilar strings.
- **L3 SQLite store/recall**: Store a fact, recall it, verify content matches.

### Integration Tests

- **Multi-turn dialog**: "Schalte den Ventilator ein" → "Mach ihn aus" → verify pronoun resolution.
- **State persistence after reset**: Trigger session reset → "Ist der Ventilator an?" → verify correct answer from entity store.
- **Multi-turn with memory**: "Merke: Mein Name ist Alex" → "Wie heiße ich?" → verify correct recall.
- **Semantic recall**: Store "Ich arbeite im Home-Office" → query "Wo ist mein Büro?" → verify hit.
- **Tool routing**: Register 20 tools → query "Ventilator" → verify only fan-related tools in prompt.
- **Persistence**: Store facts → restart service → verify facts are recalled and entity states reconstructed.

### Performance Tests

- **nucleo 50 tools**: Measure `select_tools` latency. Target: < 1 ms.
- **fastembed recall 100 facts**: Measure `recall` latency. Target: < 15 ms.
- **SQLite write**: Measure entity history write latency. Target: < 1 ms.
- **KV-cache reuse**: Measure wall-clock latency for second command vs. first. Target: 30x faster.

---

## 15. Migration Path

The implementation is backward-compatible and can be deployed in six incremental steps. Each step is independently functional.

### Step 1: Persistent LLM Worker (L0)

1. Add `LlmWorker`, `LlmWorkerCommand`, `run_worker`, `handle_generate` to `llm.rs`.
2. Replace `llm_engine` with `llm_worker` in `service.rs`.
3. Update `react.rs` to call `worker.generate()` instead of `spawn_blocking`.
4. Test: verify KV-cache reuse across commands (delta token log output).

### Step 2: Conversation History (L1)

1. Add `conversation_history` field to `VoiceAssistantService`.
2. Load/save history in `execute_react_loop`.
3. Add `max_history_messages` config field.
4. Test: multi-turn dialog with pronoun resolution.

### Step 3: Entity State Store (L2, in-memory only)

1. Add `entity_store` field to `VoiceAssistantService`.
2. Implement `extract_entity_state` function.
3. Call it after successful tool invocations in the ReAct loop.
4. Register `memory://entities` resource and `memory_query` tool.
5. Test: verify entity store updates after button tool calls.

### Step 4: nucleo Tool Router (L3, standalone)

1. Add `nucleo-matcher` dependency.
2. Create `tool_router.rs` with `ToolRouter`.
3. Modify `build_system_prompt` to split into stable prompt + `build_context_message`.
4. Use `ToolRouter` in `build_context_message`.
5. Add `max_tools_in_prompt` config field.
6. Test: verify tool filtering works with 10+ tools.

### Step 5: SQLite Persistent Store (L2 write-through + L3 facts)

1. Add `rusqlite` dependency.
2. Create `memory.rs` with SQLite schema and key-value store (no embeddings yet).
3. Register `memory_store`, `memory_recall` (keyword-based), `memory_list`, `memory_forget` MCP tools.
4. Add entity history write-through.
5. Implement entity store reconstruction on startup.
6. Test: verify facts persist across restarts, entity states reconstructed.

### Step 6: fastembed Semantic Search (L3 full)

1. Add `fastembed` dependency.
2. Extend `memory.rs` with `TextEmbedding` model and vector store.
3. Replace keyword-based `memory_recall` with semantic recall.
4. Add `{long_term}` injection in `build_context_message`.
5. Test: verify semantic recall finds related facts across language variations.

---

## 16. Habit Learning (Future Enhancement)

After accumulating entity history, the service can periodically analyze patterns:

```rust
/// Analyzes entity history for recurring patterns.
/// Called periodically (e.g., every 100 commands).
pub fn analyze_habits(&self) -> Result<Vec<Habit>, MemoryError> {
    let db = self.entity_db.lock().map_err(|_| MemoryError::DbLocked)?;

    // Find entities with regular daily patterns.
    // Group by entity + hour of day, count occurrences.
    // If an entity changes state at the same hour > 70% of days,
    // it's a habit.
    // Store detected habits as facts via semantic_memory.store().
}
```

This is a future enhancement and not part of the initial implementation.
