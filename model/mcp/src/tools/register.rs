use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_MCP_REGISTER_TOOL: &str = "mcp.register.tool";

/// Behavioral hints for an MCP tool, matching the MCP spec `ToolAnnotations`.
///
/// All fields are optional hints — clients use them for UX decisions
/// (e.g. confirmation dialogs), not as guarantees.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations {
    /// A human-readable title for the tool.
    pub title: Option<String>,
    /// If true, the tool does not modify its environment.
    /// Default: false.
    pub read_only_hint: Option<bool>,
    /// If true, the tool may perform destructive updates.
    /// Default: true.
    pub destructive_hint: Option<bool>,
    /// If true, calling the tool repeatedly with the same arguments
    /// has no additional effect.
    /// Default: false.
    pub idempotent_hint: Option<bool>,
    /// If true, the tool may interact with an open world of external entities.
    /// Default: true.
    pub open_world_hint: Option<bool>,
}

impl ToolAnnotations {
    /// Creates annotations for a read-only tool.
    pub fn read_only() -> Self {
        Self {
            read_only_hint: Some(true),
            open_world_hint: Some(false),
            ..Default::default()
        }
    }

    /// Creates annotations for a destructive, non-idempotent tool.
    pub fn destructive() -> Self {
        Self {
            destructive_hint: Some(true),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
            ..Default::default()
        }
    }

    /// Creates annotations for an idempotent tool (repeated calls are safe).
    pub fn idempotent() -> Self {
        Self {
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
            ..Default::default()
        }
    }

    /// Sets the title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the open_world_hint.
    pub fn with_open_world(mut self, open: bool) -> Self {
        self.open_world_hint = Some(open);
        self
    }
}

/// Message sent by a plugin to register a tool with the MCP server.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct RegisterToolMessage {
    /// Unique tool name exposed to MCP clients.
    pub name: stabby::string::String,
    /// Human-readable description of the tool.
    pub description: stabby::string::String,
    /// JSON schema for the tool's input parameters.
    pub input_schema: stabby::string::String,
    /// Human-readable title for UI display. None means no title set.
    pub title: stabby::option::Option<stabby::string::String>,
    /// JSON-serialized `ToolAnnotations`. None means no annotations.
    pub annotations: stabby::option::Option<stabby::string::String>,
}

impl RegisterToolMessage {
    /// Create a new tool registration message.
    pub fn new(name: &str, description: &str, input_schema: &str) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: input_schema.into(),
            title: stabby::option::Option::Some(name.into()),
            annotations: stabby::option::Option::None(),
        }
    }

    /// Sets the title for the tool.
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = stabby::option::Option::Some(title.into());
        self
    }

    /// Sets the title for the tool if `title` is `Some`.
    pub fn maybe_with_title(mut self, title: Option<&str>) -> Self {
        if let Some(title) = title {
            self.title = stabby::option::Option::Some(title.into());
        }
        self
    }

    /// Sets the annotations for the tool.
    /// Serializes `ToolAnnotations` to a JSON string.
    pub fn with_annotations(mut self, annotations: &ToolAnnotations) -> Self {
        self.annotations = stabby::option::Option::Some(serde_json::to_string(annotations).unwrap_or_default().into());
        self
    }
}

impl TypedMessage for RegisterToolMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_mcp::RegisterToolMessage");
}

impl MessageTopic for RegisterToolMessage {
    fn topic() -> &'static str {
        TOPIC_MCP_REGISTER_TOOL
    }
}

impl SharedMessage for RegisterToolMessage {
    fn topic(&self) -> &'static str {
        TOPIC_MCP_REGISTER_TOOL
    }
}

/// Description of a tool registered by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub plugin_id: String,
    /// Human-readable title for UI display. None if not set.
    pub title: Option<String>,
    /// Behavioral hints for the tool. None if not set.
    pub annotations: Option<ToolAnnotations>,
}
