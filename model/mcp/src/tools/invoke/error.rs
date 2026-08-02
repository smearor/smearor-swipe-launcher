use thiserror::Error;

#[derive(Debug, Error)]
#[error("Unknown mcp tool {name}, correlation_id: {correlation_id}")]
pub struct InvokeToolError {
    /// The tool name
    pub(crate) name: String,
    /// Correlation ID used to match the response.
    pub(crate) correlation_id: String,
}

impl InvokeToolError {
    pub fn new(e: UnknownToolError, correlation_id: &str) -> Self {
        InvokeToolError {
            name: e.0,
            correlation_id: correlation_id.to_string(),
        }
    }
}

#[derive(Debug, Error)]
#[error("Unknown mcp tool {0}")]
pub struct UnknownToolError(pub(crate) String);

impl UnknownToolError {
    pub fn new(tool: &str) -> Self {
        UnknownToolError(tool.to_string())
    }
}
