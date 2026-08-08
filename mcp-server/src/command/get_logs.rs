use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

fn default_min_level() -> String {
    "debug".to_string()
}

fn default_limit() -> usize {
    200
}

/// Parameters for retrieving launcher logs via the `launcher_get_logs` MCP tool.
///
/// `min_level` is accepted as `String` in the JSON schema (for LLM-friendly enum values)
/// but parsed to `tracing::Level` in the handler for ordinal comparison.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct GetLogsParams {
    /// Minimum log level: "trace", "debug", "info", "warn", "error".
    /// Parsed to `tracing::Level` in the handler for ordinal comparison.
    #[serde(default = "default_min_level")]
    #[builder(default = "debug".to_string())]
    pub min_level: String,
    /// Filter by tracing target prefix (e.g. "smearor_voice_assistant").
    #[serde(default)]
    #[builder(default)]
    pub target_prefix: Option<String>,
    /// Only return entries from the last N seconds.
    #[serde(default)]
    #[builder(default)]
    pub since_seconds: Option<u64>,
    /// Maximum number of entries to return (most recent N). Default: 200.
    #[serde(default = "default_limit")]
    #[builder(default = default_limit())]
    pub limit: usize,
}

impl McpCommandVariant for GetLogsParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::GetLogs(wrapper)
    }
}

impl ToolDefinitionCreator for GetLogsParams {
    fn tool_name() -> &'static str {
        "launcher_get_logs"
    }
    fn tool_description() -> &'static str {
        "Retrieves diagnostic log entries from the launcher's tracing ring buffer. Supports filtering by log level, target prefix, time window, and result limit. Useful for diagnosing failures and inspecting debug/trace output after test runs."
    }
}
