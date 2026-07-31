# Concept: Multilingual Tool Selection via BGEM3 Embeddings

This document describes the concept for replacing nucleo-based fuzzy string matching in the `ToolRouter` and `CatalogRouter` with **semantic embedding
similarity** using the BGEM3 multilingual model. This enables cross-language tool selection (e.g., German user query matching English tool names) and eliminates
the fallback-to-all-tools problem that causes `NoKvCacheSlot` errors.

---

## 1. Goal & Motivation

### Current State

The Voice Assistant uses two selection mechanisms for building the context message:

1. **ToolRouter** (`services/voice_assistant/src/tool_router.rs`): Selects up to `max_tools_in_prompt` (20) tools from the catalog using nucleo fuzzy matching +
   keyword pre-filtering.
2. **CatalogRouter** (`services/voice_assistant/src/catalog_router.rs`): Selects resources and prompts using the same nucleo approach.

Both rely on string-level matching:

- `match_text = format!("{} {}", tool.name, tool.description)` — only English text, ignores `input_schema` parameter descriptions
- `extract_keywords()` splits on whitespace and matches against the keyword index
- nucleo `Pattern::score()` computes fuzzy string similarity

### Problem

**Cross-language mismatch**: User queries are typically in German ("Ermittle den Sonnenuntergang in München"), but tool names and descriptions are in English (
"weather_get_forecast", "Get weather forecast for the configured location"). nucleo fuzzy matching fails because there is no string overlap.

**Fallback cascade**: When nucleo matching fails, the code falls back to **all tools**:

- `pre_filter_candidates()` (line 159): If no keywords match, all tools become candidates.
- `fallback_tools()` (line 169): If nucleo scores nothing, all tools are returned.

With 60 tools and full JSON schemas, this produces a context message of ~6000+ tokens. Combined with the system prompt (~1500 tokens), this exceeds
`n_ctx: 4096` and triggers `NoKvCacheSlot` even with sufficient VRAM.

**No semantic understanding**: nucleo cannot match "Lichter" (German) to `audio_set_volume` or "Wohnzimmer" to entity states. It only matches character
sequences.

### Required Capabilities

| Capability                    | Example                                    | Solution                                              |
|-------------------------------|--------------------------------------------|-------------------------------------------------------|
| Cross-language tool selection | "Sonnenuntergang" → `weather_get_forecast` | BGEM3 embedding similarity                            |
| No fallback-to-all            | 0 nucleo matches → still returns top-N     | Semantic ranking always produces scores               |
| Resource selection            | "Licht im Wohnzimmer" → `shelly://devices` | BGEM3 on resource descriptions                        |
| Prompt selection              | "memory" → `memory_guide` prompt           | BGEM3 on prompt descriptions                          |
| Low latency                   | < 50ms per selection                       | Single query embedding + vectorized cosine similarity |
| No additional model loading   | BGEM3 already loaded for `SemanticMemory`  | Reuse existing `TextEmbedding` instance               |

---

## 2. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────────┐
│                       Voice Assistant Service                            │
│                                                                          │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │  Tool Router     │    │  Catalog Router  │    │  Semantic        │   │
│  │  (Embedding)     │    │  (Embedding)     │    │  Memory          │   │
│  │                  │    │                  │    │  (existing)      │   │
│  │  tool embeddings │    │  resource embeds │    │                  │   │
│  │  (cached)        │    │  prompt embeds   │    │  TextEmbedding   │   │
│  │                  │    │  (cached)        │    │  (BGEM3, ROCm)   │   │
│  │  select_tools()  │    │  select()        │    │                  │   │
│  │  1. embed query  │    │  1. embed query  │    │  embed_single()  │   │
│  │  2. cosine sim   │    │  2. cosine sim   │    │  embed_batch()   │   │
│  │  3. top-N        │    │  3. top-N        │    │                  │   │
│  └────────┬─────────┘    └────────┬─────────┘    └────────┬─────────┘   │
│           │                       │                       │             │
│           └───────────────────────┴───────────────────────┘             │
│                                   │                                     │
│                          ┌────────▼─────────┐                          │
│                          │  EmbeddingEngine │                          │
│                          │  (shared)        │                          │
│                          │                  │                          │
│                          │  TextEmbedding   │                          │
│                          │  (BGEM3, ROCm)   │                          │
│                          │                  │                          │
│                          │  embed_single()  │                          │
│                          │  embed_batch()   │                          │
│                          └──────────────────┘                          │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. Service startup: SemanticMemory loads BGEM3 via fastembed/ONNX Runtime (ROCm)
2. Tool registration: on_tool_registered() → rebuild_tool_router()
   a. ToolRouter.rebuild() computes BGEM3 embedding for each tool's match_text
   b. Embeddings cached in ToolRouter.tool_embeddings: Vec<(Vec<f32>, ToolEntry)>
3. User query: "Ermittle den Sonnenuntergang in München..."
   a. build_context_message(user_text)
   b. select_tools_for_prompt(user_text)
      - ToolRouter.select_tools(query, top_n)
      - Embed query via EmbeddingEngine.embed_single(query) → Vec<f32>
      - Cosine similarity against all tool_embeddings
      - Sort by similarity, take top_n (20)
      - Return semantically matched tools
   c. select_resources_for_prompt(user_text) → same approach
   d. select_prompts_for_prompt(user_text) → same approach
4. Context message contains only semantically relevant tools/resources/prompts
5. LLM receives compact context → no NoKvCacheSlot
```

---

## 3. Data Model

### EmbeddingEngine

A shared embedding engine that wraps the existing `fastembed::TextEmbedding` model. This avoids loading BGEM3 twice (once for SemanticMemory, once for
ToolRouter). If the configured model (e.g. BGEM3) fails to load, the engine falls back to `BGESmallENV15Q`, mirroring `SemanticMemory::uninit()` behavior. This
ensures tool selection remains functional even if BGEM3 is unavailable.

```rust
/// Shared embedding engine wrapping the BGEM3 model.
/// Provides thread-safe embedding generation for tool/resource/prompt selection.
/// Reuses the same TextEmbedding instance as SemanticMemory when possible.
pub struct EmbeddingEngine {
    model: TextEmbedding,
    /// Cache: text string -> embedding vector.
    /// Avoids re-embedding identical text across selections.
    cache: Cache<String, Vec<f32>>,
}

impl EmbeddingEngine {
    /// Creates a new embedding engine from a fastembed model.
    pub fn new(model: TextEmbedding) -> Self {
        let cache = Cache::builder()
            .max_capacity(512)
            .time_to_live(std::time::Duration::from_secs(3600))
            .build();
        Self { model, cache }
    }

    /// Embeds a single text string, using the cache when possible.
    pub fn embed_single(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        if let Some(cached) = self.cache.get(text) {
            return Ok(cached);
        }
        let embedding = self.model
            .embed(vec![text.to_string()], None)
            .map_err(|e| EmbeddingError::Failed(e.to_string()))?
            .into_iter()
            .next()
            .ok_or(EmbeddingError::NoResult)?;
        self.cache.insert(text.to_string(), embedding.clone());
        Ok(embedding)
    }

    /// Embeds multiple texts in a single batch call.
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let embeddings = self.model
            .embed(texts.to_vec(), Some(DEFAULT_EMBED_BATCH_SIZE))
            .map_err(|e| EmbeddingError::Failed(e.to_string()))?;
        for (text, embedding) in texts.iter().zip(embeddings.iter()) {
            self.cache.insert(text.clone(), embedding.clone());
        }
        Ok(embeddings)
    }
}
```

### ToolRouter (Modified)

```rust
/// Tool router that selects tools via semantic embedding similarity.
/// Rebuilds embeddings when tools are registered/unregistered.
/// Caches query embeddings with moka to avoid repeated inference.
pub struct ToolRouter {
    /// Shared embedding engine (BGEM3).
    embedding_engine: Option<Arc<EmbeddingEngine>>,
    /// Tool entries with pre-computed embeddings.
    tools: Vec<ToolEmbeddingEntry>,
    /// Cache: query string -> selected tool entries.
    selection_cache: Cache<(String, usize), Vec<ToolCatalogEntry>>,
}

/// A tool entry with its pre-computed semantic embedding.
struct ToolEmbeddingEntry {
    name: String,
    description: String,
    input_schema: String,
    /// BGEM3 embedding of "name description input_schema".
    embedding: Vec<f32>,
}
```

### CatalogRouter (Modified)

```rust
/// Generic catalog router for resources and prompts using semantic embedding similarity.
pub struct CatalogRouter {
    /// Shared embedding engine (BGEM3).
    embedding_engine: Option<Arc<EmbeddingEngine>>,
    /// Catalog entries with pre-computed embeddings.
    entries: Vec<CatalogEmbeddingEntry>,
    /// Cache: query string -> selected serialized entries.
    selection_cache: Cache<(String, usize), Vec<String>>,
}

/// A catalog entry with its pre-computed semantic embedding.
struct CatalogEmbeddingEntry {
    /// Serialized JSON representation for the context message.
    serialized: String,
    /// BGEM3 embedding of the entry's matchable text.
    embedding: Vec<f32>,
}
```

---

## 4. Implementation Plan

### Phase 1: EmbeddingEngine

**File**: `services/voice_assistant/src/embedding_engine.rs` (new)

- Extract `EmbeddingEngine` from `SemanticMemory`'s embedding logic
- `EmbeddingEngine::new(model: TextEmbedding)` — accepts existing model
- `EmbeddingEngine::embed_single(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>`
- `EmbeddingEngine::embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError>`
- Thread-safe via `&self` (fastembed `TextEmbedding` is `Send + Sync`)
- Module declaration in `lib.rs`

### Phase 2: SemanticMemory Refactor

**File**: `services/voice_assistant/src/memory.rs`

- `SemanticMemory` internally uses `EmbeddingEngine` instead of direct `TextEmbedding`
- `SemanticMemory::embedding_engine() -> &Arc<EmbeddingEngine>` — exposes the engine for reuse
- No breaking changes to `store()`, `recall()`, `store_batch()` APIs
- The `cosine_similarity` function moves to a shared location (or stays as a free function used by both)

### Phase 3: ToolRouter Rewrite

**File**: `services/voice_assistant/src/tool_router.rs`

- Remove nucleo dependency (`nucleo_matcher`, `Matcher`, `Pattern`, `Utf32Str`)
- Remove keyword pre-filter (`extract_keywords`, `categorized_tools`, `pre_filter_candidates`)
- Remove `fallback_tools()` — no longer needed (semantic matching always produces scores)
- `rebuild(catalog, embedding_engine)`:
    1. Build `match_text = format!("{} {} {}", tool.name, tool.description, tool.input_schema)` for each tool — including the `input_schema` ensures parameter
       descriptions (e.g. `volume_level: integer`) contribute semantically valuable keywords that a bare description may omit
    2. Batch-embed all match_texts via `embedding_engine.embed_batch()` — re-embeds all tools on every rebuild; at ~100ms on ROCm for 60 tools this is
       negligible since tool registrations are rare events (startup, dynamic plugin load). Simpler and more stable than incremental append.
    3. Store as `Vec<ToolEmbeddingEntry>`
- `select_tools(query, top_n)`:
    1. Embed query via `embedding_engine.embed_single(query)`
    2. Compute cosine similarity against all `tool_embeddings`
    3. Sort by similarity descending
    4. Filter out tools below `tool_selection_threshold` (default: 0.3) — configurable in `services.toml`, empirically tunable without code changes
    5. Take `top_n` from remaining tools
    6. Always returns ≤ `top_n` tools (never all)

### Phase 4: CatalogRouter Rewrite

**File**: `services/voice_assistant/src/catalog_router.rs`

- Same approach as ToolRouter: pre-compute embeddings for resource/prompt descriptions
- `rebuild(catalog, embedding_engine)`:
    1. Build matchable text for each entry (uri + name + description for resources, name + description + arguments_schema for prompts)
    2. Batch-embed via `embedding_engine.embed_batch()`
    3. Store as `Vec<CatalogEmbeddingEntry>`
- `select(query, top_n)`:
    1. Embed query
    2. Cosine similarity against all entry embeddings
    3. Sort, take top_n
    4. Return serialized JSON strings

### Phase 5: Service Integration

**File**: `services/voice_assistant/src/service.rs`

- `VoiceAssistantService` gains `embedding_engine: Option<Arc<EmbeddingEngine>>`
- During `new()`, after `SemanticMemory` initialization:
    - Extract `Arc<EmbeddingEngine>` from `SemanticMemory`
    - Store in `service.embedding_engine`
- `rebuild_tool_router()` passes `embedding_engine` to `ToolRouter::rebuild()`
- `rebuild_resource_router()` and `rebuild_prompt_router()` pass `embedding_engine` to `CatalogRouter::rebuild()`
- If `embedding_engine` is `None` (SemanticMemory failed to load), routers fall back to empty selection (no tools) — better than all tools

### Phase 6: Config

**File**: `services/voice_assistant/src/config.rs`

- Add `tool_selection_method: ToolSelectionMethod` enum:
    - `Embedding` (default) — BGEM3 semantic similarity
    - `Nucleo` — legacy fuzzy matching (for fallback/debugging)
- Add `tool_selection_threshold: f32` — minimum cosine similarity to include a tool (default: 0.3)
- Tools below the threshold are excluded even if in top-N

**File**: `services.toml`

- `tool_selection_method = "embedding"` (or `"nucleo"` for legacy)
- `tool_selection_threshold = 0.3`

---

## 5. Performance Analysis

### Embedding Latency

| Operation                    | Count | Latency (ROCm) | Latency (CPU)  |
|------------------------------|-------|----------------|----------------|
| Tool embeddings (rebuild)    | 60    | ~100ms (batch) | ~500ms (batch) |
| Query embedding (per turn)   | 1     | ~5ms           | ~20ms          |
| Cosine similarity (60 tools) | 60    | < 0.1ms        | < 0.1ms        |
| Total per pipeline iteration | —     | ~5ms           | ~20ms          |

The rebuild cost (~100ms on ROCm) is incurred only when tools are registered/unregistered — not per query.

### Token Budget Comparison

| Scenario                        | Tools in Prompt | Estimated Tokens | Fits n_ctx=4096?   |
|---------------------------------|-----------------|------------------|--------------------|
| Current (nucleo fallback)       | 60 (all)        | ~6000            | No → NoKvCacheSlot |
| Current (nucleo match)          | 20 (top-N)      | ~2000            | Yes                |
| Proposed (BGEM3, threshold=0.3) | 5–15 (semantic) | ~500–1500        | Yes                |
| Proposed (BGEM3, top-N=20)      | 20 (semantic)   | ~2000            | Yes                |

### Memory Footprint

- BGEM3 model: already loaded for `SemanticMemory` — **no additional memory**
- Tool embeddings: 60 × 1024 dims × 4 bytes = ~240KB — negligible
- Resource/prompt embeddings: ~20 × 1024 × 4 = ~80KB — negligible

---

## 6. Fallback Strategy

### Embedding Engine Unavailable

If the configured embedding model (e.g. BGEM3) fails to load, `EmbeddingEngine` falls back to `BGESmallENV15Q` — mirroring `SemanticMemory::uninit()` behavior.
This ensures tool selection remains functional even with a smaller, less multilingual model.

If `SemanticMemory` fails to initialize entirely (model load error, ROCm unavailable):

1. `embedding_engine` is `None`
2. `ToolRouter.rebuild()` skips embedding computation, stores empty `tools`
3. `select_tools()` returns empty `Vec` — context message has `"Available tools: []"`
4. LLM receives no tools and returns `clarify` — graceful degradation
5. Log warning: "Tool selection disabled: embedding engine unavailable"

### Nucleo as Optional Fallback

For debugging or environments without GPU:

- `tool_selection_method = "nucleo"` in `services.toml` keeps the old behavior
- `ToolRouter` retains nucleo code behind a config flag
- Both paths share the same `select_tools()` interface

---

## 7. Migration Path

1. **Phase 1–2**: Extract `EmbeddingEngine` — no behavior change, pure refactor
2. **Phase 3–4**: Rewrite routers — behavior change (semantic instead of fuzzy)
3. **Phase 5**: Wire up in service — enable semantic selection
4. **Phase 6**: Config options — allow switching back to nucleo if needed

Each phase can be built and tested independently. Phase 3–4 is the breaking change; phases 1–2 and 5–6 are additive.
