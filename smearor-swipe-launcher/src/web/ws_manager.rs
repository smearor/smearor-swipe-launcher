use crate::web::web_update::WebUpdate;
use dashmap::DashMap;
use tokio::sync::broadcast;

/// Manages WebSocket connections per instance.
///
/// Each instance has a `broadcast::Sender<WebUpdate>`. When a broker message
/// is forwarded, it is sent to the matching instance's broadcast channel,
/// which delivers it to all connected WebSocket clients.
pub struct WebSocketManager {
    channels: DashMap<String, broadcast::Sender<WebUpdate>>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self { channels: DashMap::new() }
    }

    /// Register a new instance for WebSocket updates.
    pub fn register_instance(&self, instance_id: &str) {
        let (tx, _rx) = broadcast::channel::<WebUpdate>(64);
        self.channels.insert(instance_id.to_string(), tx);
    }

    /// Unregister an instance.
    pub fn unregister_instance(&self, instance_id: &str) {
        self.channels.remove(instance_id);
    }

    /// Get the broadcast sender for an instance.
    pub fn get_sender(&self, instance_id: &str) -> Option<broadcast::Sender<WebUpdate>> {
        self.channels.get(instance_id).map(|e| e.value().clone())
    }

    /// Forward a WebUpdate to all WebSocket clients of the given instance.
    pub fn broadcast(&self, update: &WebUpdate) {
        if let Some(sender) = self.get_sender(&update.instance_id) {
            let _ = sender.send(update.clone());
        }
    }
}
