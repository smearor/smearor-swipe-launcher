use crate::embedding_engine::SharedEmbeddingEngine;
use crate::embedding_engine::cosine_similarity;
use crate::service::VoiceAssistantService;
use moka::sync::Cache;
use smearor_voice_assistant_model::ToolCatalogEntry;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use tracing::debug;

/// A tool entry with its pre-computed semantic embedding.
struct ToolEmbeddingEntry {
    name: String,
    description: String,
    input_schema: String,
    /// BGEM3 embedding of "name description input_schema".
    embedding: Vec<f32>,
}

/// Tool router that selects tools via semantic embedding similarity.
/// Rebuilds embeddings when tools are registered/unregistered.
/// Caches selection results with moka to avoid repeated embedding
/// and cosine similarity computation for identical queries.
pub struct ToolRouter {
    /// Shared embedding engine (BGEM3). None when semantic memory is unavailable.
    embedding_engine: Option<SharedEmbeddingEngine>,
    /// Tool entries with pre-computed embeddings.
    tools: Vec<ToolEmbeddingEntry>,
    /// Cache: (query, top_n) -> selected tool entries.
    selection_cache: Cache<(String, usize), Vec<ToolCatalogEntry>>,
    /// Cache: (query, top_n) -> top-5 ranking with scores.
    ranking_cache: Cache<(String, usize), Vec<(String, f32)>>,
    /// Dirty flag for lazy rebuild. Set when tools are registered,
    /// checked and cleared when the router is queried.
    dirty: AtomicBool,
}

impl ToolRouter {
    /// Creates a new empty tool router.
    pub fn new() -> Self {
        let cache = Cache::builder().max_capacity(64).time_to_live(std::time::Duration::from_secs(300)).build();
        let ranking_cache = Cache::builder().max_capacity(64).time_to_live(std::time::Duration::from_secs(300)).build();
        Self {
            embedding_engine: None,
            tools: Vec::new(),
            selection_cache: cache,
            ranking_cache,
            dirty: AtomicBool::new(false),
        }
    }

    /// Rebuilds the internal tool list from the service's tool catalog.
    /// Batch-embeds all tool match texts via the embedding engine.
    /// Re-embeds all tools on every rebuild; at ~100ms on ROCm for 60 tools
    /// this is negligible since tool registrations are rare events.
    pub fn rebuild(&mut self, catalog: &[ToolCatalogEntry], engine: Option<&SharedEmbeddingEngine>) {
        self.embedding_engine = engine.cloned();
        self.selection_cache.invalidate_all();

        let Some(engine) = engine else {
            self.tools = Vec::new();
            debug!("Tool router: rebuilt with 0 tools (no embedding engine)");
            return;
        };

        let match_texts: Vec<String> = catalog.iter().map(|t| format!("{} {} {}", t.name, t.description, t.input_schema)).collect();

        let embeddings = match engine.embed_batch(&match_texts) {
            Ok(emb) => emb,
            Err(e) => {
                debug!("Tool router: embedding batch failed: {e}, skipping rebuild");
                self.tools = Vec::new();
                return;
            }
        };

        self.tools = catalog
            .iter()
            .zip(embeddings.iter())
            .map(|(t, emb)| ToolEmbeddingEntry {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
                embedding: emb.clone(),
            })
            .collect();

        debug!("Tool router: rebuilt with {} tools", self.tools.len());
        self.dirty.store(false, Ordering::Relaxed);
    }

    /// Marks the router as dirty, indicating a rebuild is needed before the next selection.
    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::Relaxed);
    }

    /// Returns true if the router has pending registrations that require a rebuild.
    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Relaxed)
    }

    /// Selects the top N tools relevant to the user query using semantic similarity.
    /// Embeds the query and computes cosine similarity against all tool embeddings.
    /// Filters out tools below the similarity threshold, then takes top_n.
    /// Always returns <= top_n tools (never all).
    pub fn select_tools(&self, query: &str, top_n: usize, threshold: f32) -> Vec<ToolCatalogEntry> {
        self.select_tools_with_ranking(query, top_n, threshold).0
    }

    /// Like `select_tools` but also returns the top-5 scored tools with their scores.
    pub fn select_tools_with_ranking(&self, query: &str, top_n: usize, threshold: f32) -> (Vec<ToolCatalogEntry>, Vec<(String, f32)>) {
        if self.tools.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let cache_key = (query.to_string(), top_n);
        if let Some(cached) = self.selection_cache.get(&cache_key) {
            debug!("Tool router: cache hit for '{}'", query);
            let ranking = self.ranking_cache.get(&cache_key).unwrap_or_default();
            return (cached, ranking);
        }

        let Some(engine) = &self.embedding_engine else {
            return (Vec::new(), Vec::new());
        };

        let query_embedding = match engine.embed_single(query) {
            Ok(emb) => emb,
            Err(e) => {
                debug!("Tool router: query embedding failed: {e}");
                return (Vec::new(), Vec::new());
            }
        };

        let mut scored: Vec<(f32, &ToolEmbeddingEntry)> = self
            .tools
            .iter()
            .map(|entry| (cosine_similarity(&query_embedding, &entry.embedding), entry))
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let ranking: Vec<(String, f32)> = scored.iter().take(5).map(|(score, entry)| (entry.name.clone(), *score)).collect();

        for (score, entry) in scored.iter().take(5) {
            debug!("Tool router: score={score:.4} name='{}' for query='{query}'", entry.name);
        }

        let selected: Vec<ToolCatalogEntry> = scored
            .into_iter()
            .filter(|(score, _)| *score >= threshold)
            .take(top_n)
            .map(|(_, entry)| ToolCatalogEntry {
                name: entry.name.clone(),
                description: entry.description.clone(),
                input_schema: entry.input_schema.clone(),
            })
            .collect();

        debug!(
            "Tool router: selected {}/{} tools for '{}' (threshold={})",
            selected.len(),
            self.tools.len(),
            query,
            threshold,
        );

        self.selection_cache.insert(cache_key.clone(), selected.clone());
        self.ranking_cache.insert(cache_key, ranking.clone());
        (selected, ranking)
    }
}

impl Default for ToolRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared tool router type used by the service.
pub type SharedToolRouter = Arc<RwLock<ToolRouter>>;

impl VoiceAssistantService {
    /// Rebuilds the tool router from the current tool catalog.
    pub fn rebuild_tool_router(&self) {
        if let (Ok(catalog), Ok(mut router)) = (self.tool_catalog.read(), self.tool_router.write()) {
            router.rebuild(&catalog, self.embedding_engine.as_ref());
        }
    }

    /// Rebuilds the resource router from the current resource catalog.
    pub fn rebuild_resource_router(&self) {
        if let (Ok(catalog), Ok(mut router)) = (self.resource_catalog.read(), self.resource_router.write()) {
            router.rebuild(&catalog, self.embedding_engine.as_ref(), |entry| {
                let serialized = serde_json::json!({
                    "uri": entry.uri,
                    "name": entry.name,
                    "description": entry.description,
                })
                .to_string();
                let match_text = format!("{} {} {}", entry.uri, entry.name, entry.description);
                (entry.uri.clone(), match_text, serialized)
            });
        }
    }

    /// Rebuilds the prompt router from the current prompt catalog.
    pub fn rebuild_prompt_router(&self) {
        if let (Ok(catalog), Ok(mut router)) = (self.prompt_catalog.read(), self.prompt_router.write()) {
            router.rebuild(&catalog, self.embedding_engine.as_ref(), |entry| {
                let serialized = serde_json::json!({
                    "name": entry.name,
                    "description": entry.description,
                    "arguments_schema": serde_json::from_str::<serde_json::Value>(&entry.arguments_schema)
                        .unwrap_or(serde_json::Value::Null),
                })
                .to_string();
                let match_text = format!("{} {} {}", entry.name, entry.description, entry.arguments_schema);
                (entry.name.clone(), match_text, serialized)
            });
        }
    }
}
