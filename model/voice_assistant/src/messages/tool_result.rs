use serde::Deserialize;
use serde::Serialize;

/// Structured error information from a tool invocation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolError {
    /// Machine-readable error code (e.g. "TOOL_NOT_FOUND", "EXECUTION_ERROR").
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// Whether retrying the same call might succeed.
    pub retryable: bool,
}

/// Structured result of a single tool invocation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolResult {
    /// Name of the tool that was invoked.
    pub tool_name: String,
    /// Whether the invocation succeeded.
    pub success: bool,
    /// The tool's output on success.
    pub result: Option<String>,
    /// Structured error information on failure.
    pub error: Option<ToolError>,
    /// Execution time in milliseconds.
    pub execution_time_ms: u64,
}

impl ToolResult {
    /// Creates a successful tool result.
    #[must_use]
    pub fn success(tool_name: &str, result: String, execution_time_ms: u64) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            success: true,
            result: Some(result),
            error: None,
            execution_time_ms,
        }
    }

    /// Creates a failed tool result with structured error info.
    #[must_use]
    pub fn failure(tool_name: &str, code: &str, message: String, retryable: bool, execution_time_ms: u64) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            success: false,
            result: None,
            error: Some(ToolError {
                code: code.to_string(),
                message,
                retryable,
            }),
            execution_time_ms,
        }
    }
}
