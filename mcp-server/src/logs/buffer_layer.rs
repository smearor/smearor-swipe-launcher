use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

use crate::logs::buffer::LogBuffer;
use crate::logs::entry::LogEntry;
use crate::logs::entry_visitor::LogEntryVisitor;

/// A `tracing_subscriber::Layer` that captures log events into a `LogBuffer`.
///
/// `on_event` constructs the `LogEntry` fully on the stack (metadata extraction,
/// visitor, timestamp), then acquires the `parking_lot::Mutex` lock only for the
/// brief `push_back`/`pop_front` operation.
pub struct LogBufferLayer {
    /// Shared reference to the `LogBuffer` where captured log events are stored.
    buffer: Arc<LogBuffer>,
}

impl LogBufferLayer {
    /// Create a new `LogBufferLayer` that writes into the given `LogBuffer`.
    pub fn new(buffer: Arc<LogBuffer>) -> Self {
        Self { buffer }
    }
}

impl<S> Layer<S> for LogBufferLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = LogEntryVisitor::default();
        event.record(&mut visitor);

        let timestamp_ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);

        let entry = LogEntry {
            timestamp_ms,
            level: *metadata.level(),
            target: metadata.target().to_string(),
            message: visitor.message,
            fields: visitor.fields,
            file: metadata.file().map(String::from),
            line: metadata.line(),
        };

        self.buffer.push(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::Level;
    use tracing_subscriber::layer::SubscriberExt;

    #[test]
    fn test_log_buffer_layer_captures_events() {
        let buffer = Arc::new(LogBuffer::new(100));
        let layer = LogBufferLayer::new(buffer.clone());

        let subscriber = tracing_subscriber::registry().with(layer);

        // Set as default subscriber for the duration of this test
        let _guard = tracing::subscriber::set_default(subscriber);

        tracing::info!(target: "test_module", "Test info message");
        tracing::warn!(target: "test_module", user_id = 42, "Warning with field");

        // Give the buffer a moment to process
        let entries = buffer.query(Some(Level::TRACE), Some("test_module"), None, None);
        assert_eq!(entries.len(), 2);

        let info_entry = entries.iter().find(|e| e.level == Level::INFO).unwrap();
        assert_eq!(info_entry.message, "Test info message");
        assert_eq!(info_entry.target, "test_module");

        let warn_entry = entries.iter().find(|e| e.level == Level::WARN).unwrap();
        assert_eq!(warn_entry.message, "Warning with field");
        assert!(warn_entry.fields.iter().any(|f| f.contains("user_id")));
    }

    #[test]
    fn test_log_buffer_layer_filters_by_level() {
        let buffer = Arc::new(LogBuffer::new(100));
        let layer = LogBufferLayer::new(buffer.clone());

        let subscriber = tracing_subscriber::registry().with(layer);
        let _guard = tracing::subscriber::set_default(subscriber);

        tracing::trace!(target: "filter_test", "trace msg");
        tracing::debug!(target: "filter_test", "debug msg");
        tracing::info!(target: "filter_test", "info msg");
        tracing::warn!(target: "filter_test", "warn msg");
        tracing::error!(target: "filter_test", "error msg");

        let all = buffer.query(Some(Level::TRACE), Some("filter_test"), None, None);
        assert_eq!(all.len(), 5);

        let warn_above = buffer.query(Some(Level::WARN), Some("filter_test"), None, None);
        assert_eq!(warn_above.len(), 2);
    }
}
