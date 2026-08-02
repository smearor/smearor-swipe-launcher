use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_MCP_REGISTER_PROMPT: &str = "mcp.register.prompt";

/// Message sent by a plugin to register a prompt with the MCP server.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct RegisterPromptMessage {
    /// Unique prompt name exposed to MCP clients.
    pub name: stabby::string::String,
    /// Human-readable description of the prompt.
    pub description: stabby::string::String,
    /// JSON schema for the prompt's arguments. Empty object if no arguments.
    pub arguments_schema: stabby::string::String,
    /// Whether the voice assistant should query memory before injecting this prompt.
    /// When true, the assistant uses `memory_query` to recall relevant facts
    /// from SemanticMemory and/or filter EntityStore entries.
    pub requires_memory: bool,
    /// Natural language query string used for SemanticMemory.recall().
    /// Ignored when `requires_memory` is false.
    /// Example: "CPU temperature threshold preference" for system_health_check.
    pub memory_query: stabby::string::String,
    /// Comma-separated entity name filter for EntityStore.
    /// Only entities whose name contains one of the filter strings are injected.
    /// Empty string means no filtering (all entities injected).
    /// Example: "cpu,memory,battery,temperature" for system_health_check.
    pub entity_filter: stabby::string::String,
}

impl RegisterPromptMessage {
    /// Create a new prompt registration message without memory requirements.
    pub fn new(name: &str, description: &str, arguments_schema: &str) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            arguments_schema: arguments_schema.into(),
            requires_memory: false,
            memory_query: "".into(),
            entity_filter: "".into(),
        }
    }

    /// Create a new prompt registration message with memory requirements.
    /// The `memory_query` is used for SemanticMemory.recall() and the
    /// `entity_filter` restricts EntityStore entries (comma-separated, empty = all).
    pub fn with_memory(name: &str, description: &str, arguments_schema: &str, memory_query: &str, entity_filter: &str) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            arguments_schema: arguments_schema.into(),
            requires_memory: true,
            memory_query: memory_query.into(),
            entity_filter: entity_filter.into(),
        }
    }
}

impl TypedMessage for RegisterPromptMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_mcp::RegisterPromptMessage");
}

impl MessageTopic for RegisterPromptMessage {
    fn topic() -> &'static str {
        TOPIC_MCP_REGISTER_PROMPT
    }
}

impl SharedMessage for RegisterPromptMessage {
    fn topic(&self) -> &'static str {
        TOPIC_MCP_REGISTER_PROMPT
    }
}

/// Description of a prompt registered by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredPrompt {
    pub name: String,
    pub description: String,
    pub arguments_schema: serde_json::Value,
    pub plugin_id: String,
    /// Whether the voice assistant should query memory before injecting this prompt.
    pub requires_memory: bool,
    /// Natural language query for SemanticMemory.recall() when requires_memory is true.
    pub memory_query: String,
    /// Comma-separated entity name filter for EntityStore. Empty means no filtering.
    pub entity_filter: String,
}
