use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

/// Errors that can occur when invoking a plugin via the message broker.
#[derive(Debug, Error)]
pub enum PluginInvokeError {
    /// The broker channel was closed and the envelope could not be sent.
    #[error("Failed to send plugin {label} invocation: {source}")]
    SendFailed {
        /// The kind of invocation (`"tool"`, `"resource"`, `"prompt"`).
        label: &'static str,
        /// The underlying send error.
        source: SendError<FfiEnvelope>,
    },
}
