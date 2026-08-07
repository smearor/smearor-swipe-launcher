use serde::Deserialize;
use serde_json::Value;

/// Request body for the `/instances/{id}/{plugin_id}/{action}` endpoint.
///
/// The `payload` field is an optional JSON object that gets forwarded to the
/// plugin's `invoke_tool` handler. This is a generic mechanism — each plugin
/// decides which actions it supports. For example, the Button plugin accepts
/// `click`, `longpress`, `swipe_up`, `swipe_down`.
#[derive(Deserialize)]
pub struct ActionRequest {
    /// Optional JSON payload forwarded to the plugin's tool handler.
    pub payload: Option<Value>,
}
