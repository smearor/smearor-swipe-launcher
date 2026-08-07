//! Tracks pending MCP tool/resource invocation responses from plugins.
//!
//! The host stores a `oneshot::Sender` for each in-flight invocation indexed by
//! correlation ID. When the plugin sends the response message, the broker routes
//! it back to the tracker, which completes the sender so the MCP server can
//! await the result.

use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Pending response channel for a single in-flight invocation.
pub type PendingResponse = oneshot::Sender<Result<String, String>>;

/// Shared pending-response map.
#[derive(Clone)]
pub struct McpResponseTracker {
    inner: Arc<DashMap<String, PendingResponse>>,
}

impl McpResponseTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    /// Register a pending response and return the receiver that will be
    /// completed when the matching response arrives.
    pub fn register(&self, correlation_id: String) -> oneshot::Receiver<Result<String, String>> {
        let (sender, receiver) = oneshot::channel::<Result<String, String>>();
        self.inner.insert(correlation_id, sender);
        receiver
    }

    /// Complete the pending response for the given correlation ID.
    pub fn resolve(&self, correlation_id: &str, result: Result<String, String>) {
        if let Some((_, sender)) = self.inner.remove(correlation_id) {
            let _ = sender.send(result);
        }
    }

    /// Cancel all pending responses with an error.
    #[allow(dead_code)]
    pub fn clear(&self, error: String) {
        let correlation_ids: Vec<String> = self.inner.iter().map(|entry| entry.key().clone()).collect();
        for correlation_id in correlation_ids {
            self.resolve(&correlation_id, Err(error.clone()));
        }
    }
}

impl Default for McpResponseTracker {
    fn default() -> Self {
        Self::new()
    }
}
