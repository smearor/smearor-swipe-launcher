use serde::Serialize;

/// A message forwarded from the broker to WebSocket clients.
///
/// Carries the topic, sender, and payload as a JSON string so the client
/// can decide how to apply the update.
#[derive(Clone, Serialize)]
pub struct WebUpdate {
    /// The target instance ID this update belongs to.
    pub instance_id: String,
    /// The broker topic that triggered this update (e.g. `area.changed`, `widget.update`).
    pub topic: String,
    /// The ID of the plugin or system component that sent the original message.
    pub sender_id: String,
    /// The message payload serialized as a JSON string.
    pub payload: String,
}
