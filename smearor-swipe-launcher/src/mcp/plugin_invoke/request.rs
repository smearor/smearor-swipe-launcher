use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use tokio::sync::mpsc::UnboundedSender;
use typed_builder::TypedBuilder;

/// Parameters for invoking a plugin tool or prompt by name.
#[derive(Debug, Clone, TypedBuilder)]
pub struct PluginInvokeRequest<'a> {
    /// The broker sender used to dispatch the invocation message.
    pub broker_sender: &'a UnboundedSender<FfiEnvelope>,
    /// Tool or prompt name.
    pub name: &'a str,
    /// Correlation ID for matching responses.
    pub correlation_id: &'a str,
    /// JSON-encoded arguments for the invocation.
    pub arguments: &'a serde_json::Value,
}
