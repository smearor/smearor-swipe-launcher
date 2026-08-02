use thiserror::Error;

/// Error returned when an unknown MCP prompt is invoked.
#[derive(Debug, Error)]
#[error("Unknown mcp prompt {name}, correlation_id: {correlation_id}")]
pub struct InvokePromptError {
    /// The prompt name
    pub(crate) name: String,
    /// Correlation ID used to match the response.
    pub(crate) correlation_id: String,
}

impl InvokePromptError {
    pub fn new(e: UnknownPromptError, correlation_id: &str) -> Self {
        InvokePromptError {
            name: e.0,
            correlation_id: correlation_id.to_string(),
        }
    }
}

/// Error returned when a prompt name cannot be parsed into a known enum variant.
#[derive(Debug, Error)]
#[error("Unknown mcp prompt {0}")]
pub struct UnknownPromptError(pub(crate) String);

impl UnknownPromptError {
    pub fn new(prompt: &str) -> Self {
        UnknownPromptError(prompt.to_string())
    }
}
