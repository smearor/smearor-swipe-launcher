use crate::McpError;

/// Result type returned by a prompt handler.
pub type PromptResult = Result<String, McpError>;
