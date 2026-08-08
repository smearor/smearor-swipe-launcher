use crate::McpCommand;
use async_channel::SendError;
use thiserror::Error;

/// Error type for MCP tool and resource operations.
#[derive(Debug, Error)]
pub enum McpError {
    /// The requested tool was not found in the registry.
    #[error("Tool {0} not found")]
    ToolNotFound(String),
    /// The requested resource was not found in the registry.
    #[error("Resource {0} not found")]
    ResourceNotFound(String),
    /// The requested prompt was not found in the registry.
    #[error("Prompt {0} not found")]
    PromptNotFound(String),
    /// The provided parameters could not be parsed or deserialized.
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    /// The command could not be sent to the launcher core.
    #[error("Failed to send command to launcher core: {0}")]
    ChannelSend(String),
    /// The launcher core dropped the response channel before responding.
    #[error("Launcher core dropped the response channel")]
    ChannelClosed,
    /// The operation did not complete within the timeout period.
    #[error("Operation timed out")]
    Timeout,
    /// The launcher core returned an error for the requested operation.
    #[error("{0}")]
    LauncherError(String),
}

impl From<SendError<McpCommand>> for McpError {
    fn from(e: SendError<McpCommand>) -> Self {
        McpError::ChannelSend(e.to_string())
    }
}
