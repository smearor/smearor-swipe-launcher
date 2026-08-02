use serde::Deserialize;
use serde::Serialize;

/// A discovered prompt from the MCP prompt registry.
/// Used internally by the service to build the LLM context message.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PromptCatalogEntry {
    /// Prompt name (e.g., "weather_summary").
    pub name: String,
    /// Human-readable description of the prompt.
    pub description: String,
    /// JSON schema for the prompt's arguments.
    pub arguments_schema: String,
    /// Whether the voice assistant should query memory before injecting this prompt.
    pub requires_memory: bool,
    /// Natural language query for SemanticMemory.recall() when requires_memory is true.
    pub memory_query: String,
    /// Comma-separated entity name filter for EntityStore. Empty means no filtering.
    pub entity_filter: String,
}
