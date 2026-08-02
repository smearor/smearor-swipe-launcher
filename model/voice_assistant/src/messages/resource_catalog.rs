use serde::Deserialize;
use serde::Serialize;

/// A discovered resource from the MCP resource registry.
/// Used internally by the service to build the LLM system prompt.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResourceCatalogEntry {
    /// Resource URI (e.g., "audio://volume").
    pub uri: String,
    /// Display name of the resource.
    pub name: String,
    /// Human-readable description of the resource.
    pub description: String,
    /// MIME type of the resource contents.
    pub mime_type: String,
}
