use crate::service::VoiceAssistantService;
use moka::sync::Cache;
use nucleo_matcher::Config;
use nucleo_matcher::Matcher;
use nucleo_matcher::Utf32Str;
use nucleo_matcher::pattern::CaseMatching;
use nucleo_matcher::pattern::Normalization;
use nucleo_matcher::pattern::Pattern;
use smearor_voice_assistant_model::ToolCatalogEntry;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::debug;

/// A tool entry used for fuzzy matching.
struct ToolEntry {
    name: String,
    description: String,
    input_schema: String,
    /// Combined matchable text: "name description" for nucleo.
    match_text: String,
}

/// Tool router that fuzzy-matches user text against the tool catalog.
/// Rebuilt when tools are registered/unregistered.
/// Caches selection results with moka to avoid repeated fuzzy matching
/// for identical queries within the same tool set.
/// Uses keyword pre-filtering to reduce candidate set before nucleo matching.
pub struct ToolRouter {
    config: Config,
    tools: Vec<ToolEntry>,
    /// Keyword-to-tool-index mapping for pre-filtering.
    /// Built during rebuild, invalidated on rebuild.
    categorized_tools: HashMap<String, Vec<usize>>,
    /// Cache: query string -> selected tool entries.
    /// Invalidated automatically on rebuild via tool count key.
    selection_cache: Cache<(String, usize), Vec<ToolCatalogEntry>>,
}

impl ToolRouter {
    /// Creates a new empty tool router.
    pub fn new() -> Self {
        let cache = Cache::builder().max_capacity(64).time_to_live(std::time::Duration::from_secs(300)).build();
        Self {
            config: Config::DEFAULT,
            tools: Vec::new(),
            categorized_tools: HashMap::new(),
            selection_cache: cache,
        }
    }

    /// Rebuilds the internal tool list from the service's tool catalog.
    /// Builds keyword index for pre-filtering and clears the selection cache.
    pub fn rebuild(&mut self, catalog: &[ToolCatalogEntry]) {
        self.tools = catalog
            .iter()
            .map(|t| {
                let match_text = format!("{} {}", t.name, t.description);
                ToolEntry {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    input_schema: t.input_schema.clone(),
                    match_text,
                }
            })
            .collect();

        // Categorize tools by keywords for pre-filtering.
        self.categorized_tools.clear();
        for (i, tool) in self.tools.iter().enumerate() {
            let keywords = extract_keywords(&tool.match_text);
            for keyword in keywords {
                self.categorized_tools.entry(keyword.to_lowercase()).or_default().push(i);
            }
        }

        self.selection_cache.invalidate_all();
        debug!("Tool router: rebuilt with {} tools, {} keyword categories", self.tools.len(), self.categorized_tools.len());
    }

    /// Selects the top N tools relevant to the user query using fuzzy matching.
    /// Uses keyword pre-filtering to reduce the candidate set before nucleo matching.
    /// Falls back to all tools if no keyword matches are found (avoid false negatives).
    /// Results are cached per (query, top_n) pair to avoid repeated matching.
    pub fn select_tools(&self, query: &str, top_n: usize) -> Vec<ToolCatalogEntry> {
        if self.tools.is_empty() {
            return Vec::new();
        }

        let cache_key = (query.to_string(), top_n);
        if let Some(cached) = self.selection_cache.get(&cache_key) {
            debug!("Tool router: cache hit for '{}'", query);
            return cached;
        }

        // Pre-filter candidates by keyword to reduce nucleo workload.
        let candidate_indices = self.pre_filter_candidates(query);

        let mut matcher = Matcher::new(self.config.clone());
        let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
        let mut buf: Vec<char> = Vec::new();

        let mut scored: Vec<(u32, &ToolEntry)> = candidate_indices
            .iter()
            .filter_map(|&index| {
                let entry = &self.tools[index];
                buf.clear();
                let haystack = Utf32Str::new(&entry.match_text, &mut buf);
                let score = pattern.score(haystack, &mut matcher)?;
                Some((score, entry))
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));

        let selected: Vec<ToolCatalogEntry> = scored
            .iter()
            .take(top_n)
            .map(|(_, entry)| ToolCatalogEntry {
                name: entry.name.clone(),
                description: entry.description.clone(),
                input_schema: entry.input_schema.clone(),
            })
            .collect();

        let result = if selected.is_empty() {
            debug!("Tool router: no nucleo matches for '{}', using fallback tools", query);
            self.fallback_tools()
        } else {
            debug!(
                "Tool router: selected {}/{} tools for '{}': {:?}",
                selected.len(),
                self.tools.len(),
                query,
                selected.iter().map(|t| &t.name).collect::<Vec<_>>()
            );
            selected
        };

        self.selection_cache.insert(cache_key, result.clone());
        result
    }

    /// Pre-filters tool candidates by matching query keywords against the
    /// categorized keyword index. Falls back to all tools if no keywords
    /// match to avoid false negatives.
    fn pre_filter_candidates(&self, query: &str) -> Vec<usize> {
        let query_keywords: HashSet<String> = extract_keywords(query).into_iter().map(|k| k.to_lowercase()).collect();

        let mut candidate_set: HashSet<usize> = HashSet::new();
        for keyword in &query_keywords {
            if let Some(indices) = self.categorized_tools.get(keyword) {
                candidate_set.extend(indices);
            }
        }

        // If no keyword matches, use all tools (avoid false negatives).
        if candidate_set.is_empty() {
            (0..self.tools.len()).collect()
        } else {
            candidate_set.into_iter().collect()
        }
    }

    /// Returns all tools when nucleo matches nothing.
    /// This ensures the LLM always has the full catalog available
    /// even when fuzzy matching fails (e.g. cross-language queries).
    fn fallback_tools(&self) -> Vec<ToolCatalogEntry> {
        self.tools
            .iter()
            .map(|t| ToolCatalogEntry {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
            })
            .collect()
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
            router.rebuild(&catalog);
        }
    }
}

/// Extracts meaningful keywords (length > 2) from text for categorization.
/// Used to build the keyword index during rebuild and to pre-filter candidates
/// during query matching.
fn extract_keywords(text: &str) -> Vec<String> {
    text.split_whitespace()
        .flat_map(|word| if word.len() > 2 { Some(word.to_lowercase()) } else { None })
        .collect()
}
