use thiserror::Error;

#[derive(Debug, Error)]
#[error("Unknown mcp resource {uri}, correlation_id: {correlation_id}")]
pub struct InvokeResourceError {
    /// The resource name
    pub(crate) uri: String,
    /// Correlation ID used to match the response.
    pub(crate) correlation_id: String,
}

impl InvokeResourceError {
    pub fn new(e: UnknownResourceError, correlation_id: &str) -> Self {
        InvokeResourceError {
            uri: e.0,
            correlation_id: correlation_id.to_string(),
        }
    }
}

#[derive(Debug, Error)]
#[error("Unknown mcp tool {0}")]
pub struct UnknownResourceError(pub(crate) String);

impl UnknownResourceError {
    pub fn new(resource: &str) -> Self {
        UnknownResourceError(resource.to_string())
    }
}
