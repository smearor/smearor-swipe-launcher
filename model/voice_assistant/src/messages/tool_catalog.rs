use serde::Deserialize;
use serde::Serialize;

/// A discovered tool from the MCP tool registry.
/// Used internally by the service to build the LLM system prompt.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolCatalogEntry {
    /// Tool name (e.g., "system_power_action").
    pub name: String,
    /// Human-readable description of the tool.
    pub description: String,
    /// JSON schema for the tool's input parameters.
    pub input_schema: String,
}
