use serde_json::Value;

use crate::McpError;

/// Result type returned by a tool handler.
pub type ToolResult = Result<Value, McpError>;
