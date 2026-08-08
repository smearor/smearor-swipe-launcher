use serde_json::Value;

/// Result type returned by a tool handler.
pub type ToolResult = Result<Value, String>;
