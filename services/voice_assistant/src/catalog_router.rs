use crate::embedding_engine::SharedEmbeddingEngine;
use crate::embedding_engine::cosine_similarity;
use moka::sync::Cache;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use tracing::debug;

/// A catalog entry with its pre-computed semantic embedding.
struct CatalogEmbeddingEntry {
    /// Identifier (URI for resources, name for prompts).
    name: String,
    /// Serialized JSON representation for the context message.
    serialized: String,
    /// BGEM3 embedding of the entry's matchable text.
    embedding: Vec<f32>,
}

/// Generic catalog router for resources and prompts using semantic embedding similarity.
/// Rebuilt when the catalog changes. Caches selection results with moka.
pub struct CatalogRouter {
    /// Shared embedding engine (BGEM3). None when semantic memory is unavailable.
    embedding_engine: Option<SharedEmbeddingEngine>,
    /// Catalog entries with pre-computed embeddings.
    entries: Vec<CatalogEmbeddingEntry>,
    /// Cache: (query, top_n) -> serialized entries.
    selection_cache: Cache<(String, usize), Vec<String>>,
    /// Cache: (query, top_n) -> top-5 ranking with scores.
    ranking_cache: Cache<(String, usize), Vec<(String, f32)>>,
    /// Dirty flag for lazy rebuild. Set when entries are registered,
    /// checked and cleared when the router is queried.
    dirty: AtomicBool,
}

impl CatalogRouter {
    /// Creates a new empty catalog router.
    pub fn new() -> Self {
        let cache = Cache::builder().max_capacity(64).time_to_live(std::time::Duration::from_secs(300)).build();
        let ranking_cache = Cache::builder().max_capacity(64).time_to_live(std::time::Duration::from_secs(300)).build();
        Self {
            embedding_engine: None,
            entries: Vec::new(),
            selection_cache: cache,
            ranking_cache,
            dirty: AtomicBool::new(false),
        }
    }

    /// Rebuilds the internal entry list from a catalog.
    /// Each entry's name, match text, and serialized form are produced by the provided function.
    /// Batch-embeds all match texts via the embedding engine.
    pub fn rebuild<T, F>(&mut self, catalog: &[T], engine: Option<&SharedEmbeddingEngine>, serialize: F)
    where
        F: Fn(&T) -> (String, String, String),
    {
        self.embedding_engine = engine.cloned();
        self.selection_cache.invalidate_all();
        self.ranking_cache.invalidate_all();

        let Some(engine) = engine else {
            self.entries = Vec::new();
            debug!("Catalog router: rebuilt with 0 entries (no embedding engine)");
            return;
        };

        let triples: Vec<(String, String, String)> = catalog.iter().map(|item| serialize(item)).collect();
        let (names, match_texts, serializeds): (Vec<String>, Vec<String>, Vec<String>) = triples.into_iter().fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut names, mut match_texts, mut serializeds), (name, match_text, serialized)| {
                names.push(name);
                match_texts.push(match_text);
                serializeds.push(serialized);
                (names, match_texts, serializeds)
            },
        );

        let embeddings = match engine.embed_batch(&match_texts) {
            Ok(emb) => emb,
            Err(e) => {
                debug!("Catalog router: embedding batch failed: {e}, skipping rebuild");
                self.entries = Vec::new();
                return;
            }
        };

        self.entries = names
            .into_iter()
            .zip(serializeds.into_iter())
            .zip(embeddings.iter())
            .map(|((name, serialized), emb)| CatalogEmbeddingEntry {
                name,
                serialized,
                embedding: emb.clone(),
            })
            .collect();

        debug!("Catalog router: rebuilt with {} entries", self.entries.len());
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

    /// Like `select` but also returns the top-5 scored entries with their scores.
    pub fn select_with_ranking(&self, query: &str, top_n: usize, threshold: f32) -> (Vec<String>, Vec<(String, f32)>) {
        if self.entries.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let cache_key = (query.to_string(), top_n);
        if let Some(cached) = self.selection_cache.get(&cache_key) {
            let ranking = self.ranking_cache.get(&cache_key).unwrap_or_default();
            return (cached, ranking);
        }

        let Some(engine) = &self.embedding_engine else {
            return (Vec::new(), Vec::new());
        };

        let query_embedding = match engine.embed_single(query) {
            Ok(emb) => emb,
            Err(e) => {
                debug!("Catalog router: query embedding failed: {e}");
                return (Vec::new(), Vec::new());
            }
        };

        let mut scored: Vec<(f32, &CatalogEmbeddingEntry)> = self
            .entries
            .iter()
            .map(|entry| (cosine_similarity(&query_embedding, &entry.embedding), entry))
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let ranking: Vec<(String, f32)> = scored.iter().take(5).map(|(score, entry)| (entry.name.clone(), *score)).collect();

        for (score, entry) in scored.iter().take(5) {
            debug!("Catalog router: score={score:.4} name='{}' for query='{query}'", entry.name);
        }

        let selected: Vec<String> = scored
            .into_iter()
            .filter(|(score, _)| *score >= threshold)
            .take(top_n)
            .map(|(_, entry)| entry.serialized.clone())
            .collect();

        debug!(
            "Catalog router: selected {}/{} entries for '{}' (threshold={})",
            selected.len(),
            self.entries.len(),
            query,
            threshold,
        );

        self.selection_cache.insert(cache_key.clone(), selected.clone());
        self.ranking_cache.insert(cache_key, ranking.clone());
        (selected, ranking)
    }
}

impl Default for CatalogRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared catalog router type used by the service.
pub type SharedCatalogRouter = Arc<RwLock<CatalogRouter>>;
