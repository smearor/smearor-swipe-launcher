# Long-Term Memory & Intelligent Context Retrieval for the Voice Assistant

This document describes the concept for a **long-term memory** system with **intelligent context retrieval** that combines semantic vector search (`fastembed`)
and fuzzy text matching (`nucleo`) to keep the LLM context window small while providing rich memory and tool selection capabilities.

---

## 1. Problem Statement

### Current Limitations

The voice assistant currently suffers from three scaling problems:

1. **No persistent memory**: After a process restart, all knowledge of user preferences, habits, and past interactions is lost.
2. **Full tool catalog injection**: All registered MCP tools (potentially 50+) are serialized into the system prompt. With 52 tools, the catalog alone
   consumes ~4000+ characters (~1000+ tokens), leaving little room for conversation history and generation.
3. **No semantic recall**: The LLM cannot retrieve relevant past facts based on meaning. If the user stored "Ich mag den Ventilator im Sommer auf Stufe 2" and
   later asks "Wie soll der Ventilator eingestellt sein?", a simple key-value lookup would miss the semantic connection.

### Required Capabilities

| Capability       | Example                                               | Solution                           |
|------------------|-------------------------------------------------------|------------------------------------|
| Persistent facts | "Merke: Mein Name ist Alex"                           | SQLite key-value store             |
| Semantic recall  | "Wie soll der Ventilator eingestellt sein?"           | `fastembed` vector similarity      |
| Tool filtering   | 52 tools → top 5 relevant for "Ventilator aus"        | `nucleo` fuzzy matching            |
| Entity history   | "Wann habe ich den Ventilator zuletzt eingeschaltet?" | SQLite entity history              |
| Habit learning   | "User schaltet Ventilator immer um 22:00 aus"         | Pattern analysis on entity history |

---

## 2. Architecture Overview

```
+---------------------+     +--------------------------+     +-----------------------+
| Voice Assistant     |---->| nucleo Tool Router       |---->| Top 5 Tools           |
| Service             |     | (fuzzy match on query)   |     | (injected in prompt)  |
|                     |     +--------------------------+     +-----------------------+
|                     |     +--------------------------+     +-----------------------+
|                     |---->| fastembed Embedding Model|---->| Vector Store          |
|                     |     | (ONNX, ~130MB, CPU)      |     | (in-memory + SQLite)  |
|                     |     +--------------------------+     +-----------------------+
|                     |              |  ^
|                     |              v  |
|                     |     +--------------------------+     +-----------------------+
|                     |---->| Semantic Recall           |---->| Top 3 Facts           |
|                     |     | (cosine similarity)       |     | (injected in prompt)  |
|                     |     +--------------------------+     +-----------------------+
|                     |     +--------------------------+     +-----------------------+
|                     |---->| SQLite Long-Term Store    |---->| memory.db (persistent)|
|                     |     | (facts, entity history)   |     | ~/.local/share/smearor|
+---------------------+     +--------------------------+     +-----------------------+
```

Three independent subsystems:

1. **nucleo Tool Router** — Fuzzy-matches the user's text against tool names and descriptions. Returns only the top N relevant tools. Reduces system prompt
   from ~4000 chars to ~500 chars.
2. **fastembed Vector Store** — Generates semantic embeddings for stored facts. Enables meaning-based recall (not just keyword matching).
3. **SQLite Persistent Store** — Stores facts, entity history, and embeddings. Survives process restarts.

---

## 3. nucleo: Intelligent Tool Router

### Why nucleo Instead of Embeddings for Tools

Tool names and descriptions are short, keyword-dense strings. Fuzzy matching is ideal because:

- **Speed**: nucleo matches against 50+ tools in **microseconds** (Smith-Waterman algorithm with aggressive prefiltering). `fastembed` would require an ONNX
  forward pass (~5-10ms per query).
- **Keyword precision**: "Ventilator aus" contains the exact word "Ventilator" which appears in the tool name `button_shelly_fan_ventilator`. Fuzzy matching
  catches this directly.
- **No model loading**: nucleo is a pure algorithm. No ONNX runtime, no model download, no RAM overhead.
- **Multilingual**: nucleo handles Unicode graphemes correctly, matching German tool descriptions.

### Data Structure

```rust
use nucleo_matcher::{Config, Matcher, Pattern, CaseMatching};

/// Tool router that fuzzy-matches user text against the tool catalog.
/// Rebuilt when tools are registered/unregistered.
pub struct ToolRouter {
    /// The nucleo matcher configuration.
    config: Config,
    /// Cached tool entries for matching.
    tools: Vec<ToolEntry>,
}

/// A tool entry with its name and description as matchable text.
struct ToolEntry {
    name: String,
    description: String,
    input_schema: String,
    /// Combined matchable text: "name description" for nucleo.
    match_text: String,
}
```

### Implementation

```rust
impl ToolRouter {
    /// Creates a new tool router with the given configuration.
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
    /// Uses nucleo's Smith-Waterman algorithm for scoring.
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

        // Sort by score descending.
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .take(top_n)
            .map(|(_, tool)| tool)
            .collect()
    }
}
```

### Integration in `build_system_prompt`

The system prompt builder is modified to accept a user query and only inject the matching tools:

```rust
pub fn build_system_prompt(&self, user_text: &str) -> String {
    let catalog = self.tool_catalog.read().unwrap_or_else(|e| e.into_inner());
    let catalog_vec = catalog.iter().collect::<Vec<_>>();

    // Use nucleo to select top N relevant tools.
    let max_tools = self.config.max_tools_in_prompt;
    let selected = self.tool_router.select_tools(user_text, max_tools);

    let tools_json: Vec<serde_json::Value> = selected
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "input_schema": serde_json::from_str::<serde_json::Value>(&t.input_schema)
                    .unwrap_or(serde_json::Value::Null),
            })
        })
        .collect();

    let serialized = serde_json::to_string(&tools_json).unwrap_or_default();

    debug!(
        "Tool router: selected {}/{} tools for query '{user_text}'",
        selected.len(),
        catalog_vec.len(),
    );

    // ... rest of prompt building (entities, long_term, template) ...
}
```

### Fallback Strategy

If nucleo returns zero matches (e.g., the user says "Hallo" with no tool intent), the system falls back to injecting a minimal set of always-available tools:

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

### Configuration

```rust
/// Maximum number of tools to inject into the system prompt after nucleo filtering.
pub max_tools_in_prompt: usize,  // default: 5
```

### Performance Impact

| Metric                              | Before (all tools)         | After (nucleo top 5)     |
|-------------------------------------|----------------------------|--------------------------|
| Tool catalog in prompt              | ~4000 chars / ~1000 tokens | ~400 chars / ~100 tokens |
| Tool selection time                 | 0 ms (no filtering)        | < 1 ms (nucleo)          |
| Context saved per command           | —                          | ~900 tokens              |
| LLM confusion from irrelevant tools | High                       | Low                      |

---

## 4. fastembed: Semantic Vector Memory

### Why fastembed Instead of Keyword Matching for Facts

Facts are natural language sentences with semantic meaning. Keyword matching fails when:

- User stores: "Ich arbeite meistens im Home-Office"
- User asks: "Wo ist mein Büro?"
- Keyword match: "Büro" ≠ "Home-Office" → miss
- Semantic match: embedding ("Wo ist mein Büro?") ≈ embedding ("Ich arbeite meistens im Home-Office") → hit

### Model Selection

| Model                                                         | Size    | Dimensions | Languages      | RAM     |
|---------------------------------------------------------------|---------|------------|----------------|---------|
| `BAAI/bge-small-en-v1.5`                                      | ~130 MB | 384        | English        | ~150 MB |
| `BAAI/bge-small-zh-v1.5`                                      | ~100 MB | 512        | Chinese        | ~120 MB |
| `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2` | ~470 MB | 384        | 50+ languages  | ~500 MB |
| `BAAI/bge-m3` (quantized)                                     | ~560 MB | 1024       | 100+ languages | ~600 MB |

**Recommended**: `BAAI/bge-small-en-v1.5` for English-only, or `paraphrase-multilingual-MiniLM-L12-v2` for German support. The multilingual model is larger but
handles German queries correctly.

For resource-constrained systems (16 GB RAM), the quantized `BGESmallENV15Q` variant is recommended — it uses INT8 quantization and requires only ~80 MB RAM.

### Data Structure

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FactCategory {
    Fact,
    Preference,
    Habit,
}
```

### Implementation

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

        let mut memory = Self {
            model,
            vectors: Vec::new(),
            db,
        };
        memory.load_vectors_from_db()?;
        Ok(memory)
    }

    /// Stores a fact with its semantic embedding.
    pub fn store(&mut self, key: &str, value: &str, category: FactCategory) -> Result<String, MemoryError> {
        let id = uuid::Uuid::new_v4().to_string();
        let embedding = self.model.embed(vec![value.to_string()], None)?
            .into_iter()
            .next()
            .ok_or(MemoryError::EmbeddingFailed)?;

        let fact = StoredFact {
            id: id.clone(),
            key: key.to_string(),
            value: value.to_string(),
            category,
            embedding: embedding.clone(),
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
        if self.vectors.is_empty() {
            return Ok(Vec::new());
        }

        let query_embedding = self.model.embed(vec![query.to_string()], None)?
            .into_iter()
            .next()
            .ok_or(MemoryError::EmbeddingFailed)?;

        // Cosine similarity search.
        let mut scored: Vec<(f32, &str)> = self
            .vectors
            .iter()
            .map(|(emb, id)| {
                let similarity = cosine_similarity(&query_embedding, emb);
                (similarity, id.as_str())
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let top_ids: Vec<&str> = scored.iter().take(top_n).map(|(_, id)| *id).collect();
        let facts = self.load_facts_by_ids(&top_ids)?;

        // Update access timestamps.
        for fact in &facts {
            self.touch_fact(&fact.id)?;
        }

        Ok(facts)
    }
}

/// Computes cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
```

### Performance Characteristics

| Operation                  | Time     | Notes                                 |
|----------------------------|----------|---------------------------------------|
| Model load (startup)       | ~500 ms  | One-time cost, cached after first run |
| Embed single query         | ~5-10 ms | ONNX forward pass on CPU              |
| Store fact + embedding     | ~10 ms   | Embed + SQLite write                  |
| Recall (100 facts, top 3)  | ~10 ms   | Embed query + cosine similarity       |
| Recall (1000 facts, top 3) | ~15 ms   | Linear scan, still fast               |
| RAM (model + 1000 facts)   | ~170 MB  | ~80 MB model + ~90 MB vectors         |

---

## 5. SQLite Schema

```sql
CREATE TABLE IF NOT EXISTS facts (
    id TEXT PRIMARY KEY,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'fact',
    embedding BLOB NOT NULL,          -- serialized Vec<f32> as little-endian f32 bytes
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
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}
```

---

## 6. MCP Tools for Memory

### Tool Registration

Four MCP tools are registered for LLM-driven memory operations:

```rust
pub fn register_memory_tools(&self) {
    let broadcaster = self.get_broadcaster();

    let store_tool = RegisterToolMessage::new(
        "memory_store",
        "Store a fact or preference in long-term memory. The fact will be available for future queries.",
        r#"{ "type": "object", "properties": { "key": { "type": "string", "description": "Short identifier (e.g., 'fan_preference')" }, "value": { "type": "string", "description": "The fact to remember (e.g., 'User prefers fan on level 2 in summer')" }, "category": { "type": "string", "enum": ["fact", "preference", "habit"], "description": "Category of the stored fact" } }, "required": ["key", "value"] }"#,
    );
    broadcaster.broadcast_message_to_topic(store_tool);

    let recall_tool = RegisterToolMessage::new(
        "memory_recall",
        "Recall facts from long-term memory using semantic search. Pass a natural language query.",
        r#"{ "type": "object", "properties": { "query": { "type": "string", "description": "Natural language query (e.g., 'fan preferences')" } }, "required": ["query"] }"#,
    );
    broadcaster.broadcast_message_to_topic(recall_tool);

    let list_tool = RegisterToolMessage::new(
        "memory_list",
        "List all stored fact keys in long-term memory.",
        r#"{ "type": "object", "properties": { "category": { "type": "string", "enum": ["fact", "preference", "habit"], "description": "Optional category filter" } }, "required": [] }"#,
    );
    broadcaster.broadcast_message_to_topic(list_tool);

    let forget_tool = RegisterToolMessage::new(
        "memory_forget",
        "Delete a fact from long-term memory by key.",
        r#"{ "type": "object", "properties": { "key": { "type": "string", "description": "The key of the fact to forget" } }, "required": ["key"] }"#,
    );
    broadcaster.broadcast_message_to_topic(forget_tool);
}
```

### Tool Handlers

```rust
"memory_store" => {
let args: serde_json::Value = serde_json::from_str( & message.0.arguments.to_string())
.unwrap_or(serde_json::Value::Null);
let key = args.get("key").and_then( | v | v.as_str()).unwrap_or("");
let value = args.get("value").and_then( | v | v.as_str()).unwrap_or("");
let category = args.get("category").and_then( | v| v.as_str()).unwrap_or("fact");

match self.semantic_memory.write() {
Ok( mut memory) => {
let category = match category {
"preference" => FactCategory::Preference,
"habit" => FactCategory::Habit,
_ => FactCategory::Fact,
};
let id = memory.store(key, value, category)
.map_err( | e | format ! ("Failed to store fact: {e}"));
match id {
Ok(id) => InvokeToolResponse::success( &message.0.correlation_id, & format ! ("Stored fact '{key}' with id {id}")),
Err(e) => InvokeToolResponse::error( & message.0.correlation_id, & e),
}
}
Err(_) => InvokeToolResponse::error( &message.0.correlation_id, "Memory store locked"),
}
}

"memory_recall" => {
let args: serde_json::Value = serde_json::from_str( & message.0.arguments.to_string())
.unwrap_or(serde_json::Value::Null);
let query = args.get("query").and_then( | v | v.as_str()).unwrap_or("");

match self.semantic_memory.write() {
Ok( mut memory) => {
match memory.recall(query, 3) {
Ok(facts) => {
let json = serde_json::json ! ({
"facts": facts.iter().map(| f | serde_json::json ! ({
"key": f.key,
"value": f.value,
"category": f.category,
"last_accessed": f.last_accessed,
})).collect::< Vec < _ >> (),
});
InvokeToolResponse::success( & message.0.correlation_id, & json.to_string())
}
Err(e) => InvokeToolResponse::error( &message.0.correlation_id, & format ! ("Recall failed: {e}")),
}
}
Err(_) => InvokeToolResponse::error( &message.0.correlation_id, "Memory store locked"),
}
}
```

---

## 7. System Prompt Injection

### Reduced Prompt with All Three Placeholders

```toml
[voice_assistant]
system_prompt = "Du bist ein Smart-Home-Assistent. Verfügbare Tools: {tools}. Bekannte Geräte: {entities}. Gewusste Fakten: {long_term}. Antworte in JSON. Tool-Aufruf: {\"tool\": \"<name>\", \"arguments\": {...}}. Finale Antwort: {\"final_answer\": \"<text>\"}."
```

### Dynamic Prompt Building

```rust
pub fn build_system_prompt(&self, user_text: &str) -> String {
    // 1. nucleo: select top 5 relevant tools.
    let selected_tools = self.tool_router.select_tools(user_text, self.config.max_tools_in_prompt);
    let tools_json = self.serialize_tools(&selected_tools);

    // 2. Entity states (from short-term memory).
    let entity_summary = self.build_entity_summary();

    // 3. fastembed: semantic recall of top 3 relevant facts.
    let long_term_summary = self.build_long_term_summary(user_text);

    let template = self.config.system_prompt.as_deref().unwrap_or(DEFAULT_PROMPT);
    template
        .replace("{tools}", &tools_json)
        .replace("{entities}", &entity_summary)
        .replace("{long_term}", &long_term_summary)
}
```

### Long-Term Summary Builder

```rust
fn build_long_term_summary(&self, user_text: &str) -> String {
    if !self.config.inject_long_term_facts {
        return String::new();
    }

    let facts = self
        .semantic_memory
        .write()
        .ok()
        .and_then(|mut memory| memory.recall(user_text, 3).ok())
        .unwrap_or_default();

    if facts.is_empty() {
        return String::new();
    }

    let summary = facts
        .iter()
        .map(|f| format!("- {}: {}", f.key, f.value))
        .collect::<Vec<_>>()
        .join("\n");

    format!("\n{summary}")
}
```

### Token Budget with nucleo + fastembed

| Source                         | Before       | After                          |
|--------------------------------|--------------|--------------------------------|
| Tool catalog (52 tools)        | ~1000 tokens | ~100 tokens (top 5 via nucleo) |
| Entity states (10 devices)     | ~100 tokens  | ~100 tokens (unchanged)        |
| Long-term facts (top 3)        | —            | ~50 tokens (via fastembed)     |
| Conversation history (10 msgs) | ~500 tokens  | ~500 tokens (unchanged)        |
| **Total memory overhead**      | ~1600 tokens | ~750 tokens                    |
| **Free for generation**        | ~6592 tokens | ~7442 tokens                   |

The nucleo tool router alone saves ~900 tokens per command. Combined with fastembed's targeted fact injection, the system prompt stays compact even with 100+
tools and 1000+ stored facts.

---

## 8. Entity History Persistence

### Write-Through from Short-Term to Long-Term

When the short-term entity store updates (see `SPEAK_SWIPER_SHORT_MEMORY_CONCEPT.md`), the change is also written to the SQLite entity history:

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
                rusqlite::params![
                    state.name,
                    state.state,
                    state.last_action,
                    state.tool,
                    state.last_changed,
                ],
            );
        }
    }
}
```

### Reconstruction on Startup

When the service starts, the short-term entity store is reconstructed from the most recent entity history entries:

```rust
fn reconstruct_entity_store(db: &rusqlite::Connection) -> HashMap<String, EntityState> {
    let mut store = HashMap::new();

    let mut stmt = db
        .prepare("SELECT entity, state, action, tool, timestamp FROM entity_history ORDER BY timestamp DESC")
        .unwrap();

    let rows = stmt.query_map([], |row| {
        Ok(EntityState {
            name: row.get(0)?,
            state: row.get(1)?,
            last_action: row.get(2)?,
            tool: row.get(3)?,
            last_changed: row.get(4)?,
        })
    }).unwrap();

    for row in rows {
        if let Ok(state) = row {
            // Only keep the latest state per entity.
            if !store.contains_key(&state.tool) {
                store.insert(state.tool.clone(), state);
            }
        }
    }

    store
}
```

---

## 9. Habit Learning (Future Enhancement)

### Pattern Detection

After accumulating entity history, the service can periodically analyze patterns:

```rust
/// Analyzes entity history for recurring patterns.
/// Called periodically (e.g., every 100 commands).
pub fn analyze_habits(&self) -> Result<Vec<Habit>, MemoryError> {
    let db = self.entity_db.lock().map_err(|_| MemoryError::DbLocked)?;

    // Find entities with regular daily patterns.
    let mut stmt = db.prepare(
        "SELECT entity, state, action, timestamp
         FROM entity_history
         WHERE timestamp > datetime('now', '-30 days')
         ORDER BY timestamp"
    )?;

    // Group by entity + hour of day, count occurrences.
    // If an entity changes state at the same hour > 70% of days,
    // it's a habit.
    // ...

    // Store detected habits as facts.
    for habit in detected_habits {
        self.semantic_memory.write()?.store(
            &format!("habit_{}", habit.entity),
            &habit.description,
            FactCategory::Habit,
        )?;
    }
}
```

This is a future enhancement and not part of the initial implementation.

---

## 10. Configuration

New fields in `VoiceAssistantServiceConfig`:

```rust
/// Path to the SQLite database file for long-term memory.
pub memory_db_path: String,  // default: "~/.local/share/smearor/memory.db"

/// Maximum number of tools to inject into the system prompt after nucleo filtering.
pub max_tools_in_prompt: usize,  // default: 5

/// Whether to inject semantically recalled facts into the system prompt.
pub inject_long_term_facts: bool,  // default: true

/// Number of facts to recall via fastembed for prompt injection.
pub max_recalled_facts: usize,  // default: 3

/// Embedding model to use for semantic memory.
pub embedding_model: String,  // default: "bge-small-en-v1.5-q"
```

**TOML usage:**

```toml
[voice_assistant]
memory_db_path = "~/.local/share/smearor/memory.db"
max_tools_in_prompt = 5
inject_long_term_facts = true
max_recalled_facts = 3
embedding_model = "bge-small-en-v1.5-q"
```

---

## 11. Dependencies

### New Crate Dependencies

| Crate            | Version | Purpose             | RAM Impact                   |
|------------------|---------|---------------------|------------------------------|
| `nucleo-matcher` | 0.3     | Fuzzy tool routing  | ~0 MB (pure algorithm)       |
| `fastembed`      | 5.17    | Semantic embeddings | ~80-150 MB (model dependent) |
| `rusqlite`       | 0.31    | SQLite persistence  | ~2 MB                        |
| `chrono`         | 0.4     | Timestamps          | ~0 MB (already used)         |
| `uuid`           | 1.0     | Fact IDs            | ~0 MB (already used)         |

### No New Model Crate

All memory types are internal to the voice assistant service. No FFI sharing is required.

---

## 12. Interaction with Other Concepts

### With Context Overflow (Persistent Worker)

**nucleo tool routing reduces reset frequency**: Since the tool catalog portion of the system prompt changes with every query (different top 5 tools), the
worker must use the segmented reset strategy described in the overflow concept. Only the `{tools}` segment triggers a reset when the tool catalog itself changes
(tools registered/unregistered), not when nucleo selects different tools for different queries.

**Wait — this is a critical design decision**: If nucleo selects different tools per query, the `{tools}` portion changes every command, which would trigger a
worker reset every time. Two solutions:

- **(a) Worker ignores `{tools}` changes**: The worker only resets when the full tool catalog (all registered tools) changes, not when the injected subset
  changes. The KV cache retains the old tool subset, and the new subset is processed as delta tokens. This works because the system prompt is rebuilt from
  scratch each time anyway.
- **(b) Inject tools as a user message**: Instead of putting tools in the system prompt, send them as the first user message. The system prompt stays stable (
  "You are a desktop assistant..."). The worker only resets when the system prompt text itself changes.

**Recommendation**: Option (b) — move `{tools}` from system prompt to a context user message. This keeps the system prompt stable across commands, maximizing
KV-cache reuse.

### With Short-Term Memory

**Entity store feeds entity history**: Short-term entity updates write-through to SQLite. On startup, the short-term store is reconstructed from SQLite.

**Conversation history is not persisted**: Only entity states and facts are persisted. Conversation history (Layer 1 of short-term memory) remains volatile.

**Combined token budget**:

| Source                                  | Tokens   |
|-----------------------------------------|----------|
| System prompt (stable, no tools)        | ~50      |
| Context user message (tools via nucleo) | ~100     |
| Entity states                           | ~100     |
| Long-term facts (via fastembed)         | ~50      |
| Conversation history (10 msgs)          | ~500     |
| **Total**                               | **~800** |
| Free for generation                     | ~7392    |

### Combined Pipeline Flow

```
User says: "Mach den Ventilator aus, aber merk dir dass ich ihn abends immer ausschalte"

1. STT → "Mach den Ventilator aus, aber merk dir dass ich ihn abends immer ausschalte"
2. nucleo: select top 5 tools for "Ventilator aus" → [button_shelly_fan_ventilator, ...]
3. fastembed: recall top 3 facts for "Ventilator abends ausschalten" → [fact: "fan_preference: level 2 in summer"]
4. build_system_prompt:
   - System prompt: "Du bist ein Smart-Home-Assistent..."
   - Context message: "Tools: [button_shelly_fan_ventilator, ...]. Entities: Ventilator: on. Facts: fan_preference: level 2 in summer."
5. LLM ReAct iteration 1:
   - Tool call: button_shelly_fan_ventilator(action=longpress)
   - Entity store updated: Ventilator → off
   - Entity history written to SQLite
6. LLM ReAct iteration 2:
   - Tool call: memory_store(key="fan_evening_off", value="User schaltet den Ventilator abends immer aus", category="habit")
   - Fact stored in SQLite with embedding
7. LLM ReAct iteration 3:
   - Final answer: "Ventilator ausgeschaltet. Ich habe mir gemerkt, dass du ihn abends immer ausschaltest."
```

---

## 13. Testing Strategy

### Unit Tests

- **nucleo tool router**: Test `select_tools` with various queries. Verify top 5 results are relevant.
- **nucleo empty match**: Verify fallback to always-available tools.
- **fastembed embedding**: Verify embeddings are generated and have correct dimensions.
- **Cosine similarity**: Test with known similar and dissimilar strings.
- **SQLite store/recall**: Store a fact, recall it, verify content matches.
- **Entity history**: Write entity state, read back, verify.
- **Entity reconstruction**: Seed SQLite with history, reconstruct store, verify latest states.

### Integration Tests

- **Multi-turn with memory**: "Merke: Mein Name ist Alex" → "Wie heiße ich?" → verify correct recall.
- **Semantic recall**: Store "Ich arbeite im Home-Office" → query "Wo ist mein Büro?" → verify hit.
- **Tool routing**: Register 20 tools → query "Ventilator" → verify only fan-related tools in prompt.
- **Persistence**: Store facts → restart service → verify facts are recalled.

### Performance Tests

- **nucleo 50 tools**: Measure `select_tools` latency. Target: < 1 ms.
- **fastembed recall 100 facts**: Measure `recall` latency. Target: < 15 ms.
- **SQLite write**: Measure entity history write latency. Target: < 1 ms.

---

## 14. Affected Files

| File                                                     | Change                                                                                                          |
|----------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------|
| `services/voice_assistant/src/tool_router.rs`            | **New file**: `ToolRouter` with nucleo matching.                                                                |
| `services/voice_assistant/src/memory.rs`                 | **New file**: `SemanticMemory` with fastembed + SQLite.                                                         |
| `services/voice_assistant/src/tool_catalog.rs`           | Modify `build_system_prompt` to accept `user_text`, use `ToolRouter`, inject `{long_term}`.                     |
| `services/voice_assistant/src/mcp.rs`                    | Register `memory_store`, `memory_recall`, `memory_list`, `memory_forget` tools. Add tool handlers.              |
| `services/voice_assistant/src/react.rs`                  | Pass `user_text` to `build_system_prompt`. Write entity history after tool calls.                               |
| `services/voice_assistant/src/service/loaded_service.rs` | Add `tool_router`, `semantic_memory`, `entity_db` fields. Initialize in `new()`.                                |
| `services/voice_assistant/src/config.rs`                 | Add `memory_db_path`, `max_tools_in_prompt`, `inject_long_term_facts`, `max_recalled_facts`, `embedding_model`. |
| `services/voice_assistant/Cargo.toml`                    | Add `nucleo-matcher`, `fastembed`, `rusqlite`, `chrono` dependencies.                                           |

---

## 15. Migration Path

### Step 1: nucleo Tool Router (standalone, no other dependencies)

1. Add `nucleo-matcher` dependency.
2. Create `tool_router.rs` with `ToolRouter`.
3. Modify `build_system_prompt` to accept `user_text` and use `ToolRouter`.
4. Add `max_tools_in_prompt` config field.
5. Test: verify tool filtering works with 10+ tools.

### Step 2: SQLite Persistent Store (no fastembed yet)

1. Add `rusqlite` dependency.
2. Create `memory.rs` with SQLite schema and key-value store (no embeddings).
3. Register `memory_store`, `memory_recall` (keyword-based), `memory_list`, `memory_forget` MCP tools.
4. Add entity history write-through.
5. Test: verify facts persist across restarts.

### Step 3: fastembed Semantic Search

1. Add `fastembed` dependency.
2. Extend `memory.rs` with `TextEmbedding` model and vector store.
3. Replace keyword-based `memory_recall` with semantic recall.
4. Add `{long_term}` prompt injection.
5. Test: verify semantic recall finds related facts.

### Step 4: Entity Store Reconstruction

1. Implement `reconstruct_entity_store` from SQLite on startup.
2. Test: verify entity states survive restarts.

### Step 5: Prompt Restructuring (for KV-cache optimization)

1. Move `{tools}` from system prompt to context user message.
2. Keep system prompt stable across commands.
3. Test: verify persistent worker benefits from stable system prompt.

Each step is independently deployable and backward-compatible.
