use moka::sync::Cache;
use smearor_voice_assistant_model::ToolResult;
use std::collections::BTreeMap;
use std::time::Duration;
use tracing::debug;

/// Cache key combining tool name and deterministic parameter serialization.
fn cache_key(tool: &str, args: &serde_json::Value) -> String {
    let params_str = deterministic_json_string(args);
    format!("{tool}:{params_str}")
}

/// Converts a JSON value to a deterministic string with sorted keys.
/// This ensures that semantically identical objects with different key
/// ordering produce the same cache key.
fn deterministic_json_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            let mut sorted_map = BTreeMap::new();
            for (key, val) in map {
                sorted_map.insert(key.clone(), deterministic_json_string(val));
            }
            serde_json::to_string(&sorted_map).unwrap_or_default()
        }
        serde_json::Value::Array(arr) => {
            let sorted_arr: Vec<String> = arr.iter().map(deterministic_json_string).collect();
            serde_json::to_string(&sorted_arr).unwrap_or_default()
        }
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

/// Caches tool invocation results with TTL and LRU eviction.
///
/// Uses deterministic cache keys (tool name + sorted JSON parameters)
/// to avoid cache misses from JSON key ordering differences.
pub struct ToolCache {
    cache: Cache<String, ToolResult>,
}

impl ToolCache {
    /// Creates a new tool cache with 5-minute TTL and 1000-entry max capacity.
    #[must_use]
    pub fn new() -> Self {
        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(300))
            .time_to_idle(Duration::from_secs(60))
            .max_capacity(1000)
            .build();
        Self { cache }
    }

    /// Returns the cached result for the given tool and arguments, if present.
    pub fn get(&self, tool: &str, args: &serde_json::Value) -> Option<ToolResult> {
        let key = cache_key(tool, args);
        self.cache.get(&key)
    }

    /// Inserts a result into the cache.
    pub fn insert(&self, tool: &str, args: &serde_json::Value, result: ToolResult) {
        let key = cache_key(tool, args);
        self.cache.insert(key, result);
    }

    /// Invalidates all cache entries for a specific tool.
    #[allow(dead_code)]
    pub fn invalidate_tool(&self, tool_name: &str) {
        let prefix = format!("{tool_name}:");
        let keys_to_remove: Vec<String> = self
            .cache
            .iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .map(|(key, _)| (*key).clone())
            .collect();

        for key in keys_to_remove {
            self.cache.invalidate(&key);
        }
        debug!("Tool cache: invalidated entries for tool '{}'", tool_name);
    }

    /// Invalidates a specific cache entry.
    #[allow(dead_code)]
    pub fn invalidate_entry(&self, tool: &str, args: &serde_json::Value) {
        let key = cache_key(tool, args);
        self.cache.invalidate(&key);
    }

    /// Removes all entries from the cache.
    pub fn invalidate_all(&self) {
        self.cache.invalidate_all();
        debug!("Tool cache: invalidated all entries");
    }
}

impl Default for ToolCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Classifies an `AssistantError` into a machine-readable error code.
pub fn classify_error(error: &crate::react::AssistantError) -> String {
    match error {
        crate::react::AssistantError::ToolInvocation(_) => "EXECUTION_ERROR".to_string(),
        crate::react::AssistantError::ToolTimeout(_) => "TIMEOUT".to_string(),
        crate::react::AssistantError::Parse(_) => "PARSE_ERROR".to_string(),
        crate::react::AssistantError::LlmInference(_) => "LLM_ERROR".to_string(),
        crate::react::AssistantError::MaxIterationsReached => "MAX_ITERATIONS".to_string(),
    }
}

/// Determines whether an error is retryable.
/// Timeouts and execution errors may succeed on retry; parse errors and
/// max-iterations are not retryable.
pub fn is_retryable(error: &crate::react::AssistantError) -> bool {
    matches!(error, crate::react::AssistantError::ToolInvocation(_) | crate::react::AssistantError::ToolTimeout(_))
}

/// Creates a `ToolResult` from an `AssistantError`.
pub fn error_to_tool_result(tool_name: &str, error: &crate::react::AssistantError, execution_time_ms: u64) -> ToolResult {
    ToolResult::failure(tool_name, &classify_error(error), error.to_string(), is_retryable(error), execution_time_ms)
}
