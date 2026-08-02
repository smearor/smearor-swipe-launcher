use crate::InvokeResourceError;
use crate::InvokeResourceMessage;
use crate::InvokeResourceResponse;
use crate::UnknownResourceError;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use std::str::FromStr;

/// Trait for handling MCP resource invocations with automatic URI parsing and error handling.
///
/// Services implement `get_response` to build the response for each known resource variant.
/// The default `handle_invoke_resource_message` parses the URI, calls `get_response` for known
/// resources, and sends an error response for unknown ones.
///
/// Override `on_unknown_resource` to change error handling behavior (e.g. silent ignore).
/// Override `send_resource_response` to use a different response delivery mechanism.
pub trait McpResourceHandler<McpResources>: MessageBroadcaster
where
    McpResources: FromStr<Err = UnknownResourceError>,
{
    /// Default handler that parses the URI, dispatches to `get_response`, and sends the result.
    fn handle_invoke_resource_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        let correlation_id = message.0.correlation_id.to_string();
        let resource = match McpResources::from_str(&message.0.uri) {
            Ok(resource) => resource,
            Err(e) => {
                if let Some(response) = self.on_unknown_resource(&correlation_id, e) {
                    self.send_resource_response(response, sender_id);
                }
                return;
            }
        };
        let request = ResourceRequest::new(resource, &correlation_id, sender_id);
        let response = self.get_response(&request);
        self.send_resource_response(response, sender_id);
    }

    /// Build the response for a known resource variant.
    ///
    /// The `uri` parameter is provided for resources that need to extract query parameters
    /// or use the original URI string for serialization.
    fn get_response(&self, request: &ResourceRequest<McpResources>) -> InvokeResourceResponse;

    /// Called when the URI does not match any known resource variant.
    ///
    /// Default implementation returns an error response. Override to return `None`
    /// to silently ignore unknown resources (e.g. for services that delegate to the launcher core).
    fn on_unknown_resource(&self, correlation_id: &str, error: UnknownResourceError) -> Option<InvokeResourceResponse> {
        Some(InvokeResourceResponse::from(InvokeResourceError::new(error, correlation_id)))
    }

    /// Send the response to the message bus.
    ///
    /// Default implementation broadcasts via `MessageBroadcaster`. Override to use
    /// a direct `send_response` mechanism when the service requires `sender_id`.
    fn send_resource_response(&self, response: InvokeResourceResponse, _sender_id: &str) {
        self.get_broadcaster().broadcast_message_to_topic(response);
    }
}

pub struct ResourceRequest<'a, R> {
    pub resource: R,
    pub correlation_id: &'a str,
    pub sender_id: &'a str,
}

impl<'a, R> ResourceRequest<'a, R> {
    pub fn new(resource: R, correlation_id: &'a str, sender_id: &'a str) -> Self {
        Self {
            resource,
            correlation_id,
            sender_id,
        }
    }
}
