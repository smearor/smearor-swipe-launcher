use std::collections::VecDeque;

use parking_lot::Mutex;
use tracing::Level;

use crate::logs::entry::LogEntry;

/// Thread-safe ring buffer for capturing tracing log events.
///
/// Uses `parking_lot::Mutex` for performance in the tracing hot path.
/// `push()` evicts oldest entry at capacity. `query()` filters by level,
/// target prefix, timestamp, and limit.
#[derive(Debug)]
pub struct LogBuffer {
    /// Inner ring buffer protected by `parking_lot::Mutex` for concurrent access.
    inner: Mutex<VecDeque<LogEntry>>,
    /// Maximum number of entries the buffer can hold before evicting the oldest.
    capacity: usize,
}

impl LogBuffer {
    /// Create a new `LogBuffer` with the given bounded capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
        }
    }

    /// Push a new entry into the ring buffer, evicting the oldest if at capacity.
    pub fn push(&self, entry: LogEntry) {
        let mut guard = self.inner.lock();
        if guard.len() >= self.capacity {
            guard.pop_front();
        }
        guard.push_back(entry);
    }

    /// Query the buffer with optional filters.
    ///
    /// - `min_level`: Only entries with `level <= min_level` (higher or equal priority).
    /// - `target_prefix`: Only entries whose target starts with the given prefix.
    /// - `since_ms`: Only entries with `timestamp_ms >= since_ms`.
    /// - `limit`: Maximum number of entries to return (most recent N).
    ///
    /// Iterates backwards to collect the most recent matching entries,
    /// then reverses the result to restore chronological order.
    pub fn query(&self, min_level: Option<Level>, target_prefix: Option<&str>, since_ms: Option<u64>, limit: Option<usize>) -> Vec<LogEntry> {
        let guard = self.inner.lock();
        let max = limit.unwrap_or(usize::MAX);
        let mut results: Vec<LogEntry> = guard
            .iter()
            .rev()
            .filter(|entry| {
                if let Some(req_level) = min_level {
                    if entry.level > req_level {
                        return false;
                    }
                }
                if let Some(prefix) = target_prefix {
                    if !entry.target.starts_with(prefix) {
                        return false;
                    }
                }
                if let Some(since) = since_ms {
                    if entry.timestamp_ms < since {
                        return false;
                    }
                }
                true
            })
            .take(max)
            .cloned()
            .collect();
        results.reverse();
        results
    }

    /// Clear all entries from the buffer.
    pub fn clear(&self) {
        self.inner.lock().clear();
    }

    /// Current number of entries in the buffer.
    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    /// Maximum number of entries the buffer can hold.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::entry::LogEntry;
    use tracing::Level;

    fn make_entry(timestamp_ms: u64, level: Level, target: &str, message: &str) -> LogEntry {
        LogEntry {
            timestamp_ms,
            level,
            target: target.to_string(),
            message: message.to_string(),
            fields: Vec::new(),
            file: None,
            line: None,
        }
    }

    #[test]
    fn test_push_and_len() {
        let buffer = LogBuffer::new(100);
        assert_eq!(buffer.len(), 0);

        buffer.push(make_entry(100, Level::INFO, "test_module", "Hello"));
        buffer.push(make_entry(200, Level::DEBUG, "test_module", "World"));
        assert_eq!(buffer.len(), 2);
    }

    #[test]
    fn test_capacity() {
        let buffer = LogBuffer::new(42);
        assert_eq!(buffer.capacity(), 42);
    }

    #[test]
    fn test_query_returns_all_when_no_filters() {
        let buffer = LogBuffer::new(100);
        buffer.push(make_entry(100, Level::INFO, "mod", "a"));
        buffer.push(make_entry(200, Level::DEBUG, "mod", "b"));
        buffer.push(make_entry(300, Level::ERROR, "mod", "c"));

        let results = buffer.query(None, None, None, None);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].message, "a");
        assert_eq!(results[2].message, "c");
    }

    #[test]
    fn test_min_level_filter_excludes_lower_levels() {
        let buffer = LogBuffer::new(100);
        buffer.push(make_entry(100, Level::TRACE, "mod", "trace"));
        buffer.push(make_entry(200, Level::DEBUG, "mod", "debug"));
        buffer.push(make_entry(300, Level::INFO, "mod", "info"));
        buffer.push(make_entry(400, Level::WARN, "mod", "warn"));
        buffer.push(make_entry(500, Level::ERROR, "mod", "error"));

        let results = buffer.query(Some(Level::INFO), None, None, None);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].message, "info");
        assert_eq!(results[1].message, "warn");
        assert_eq!(results[2].message, "error");
    }

    #[test]
    fn test_target_prefix_filter() {
        let buffer = LogBuffer::new(100);
        buffer.push(make_entry(100, Level::INFO, "smearor_voice_assistant::service", "a"));
        buffer.push(make_entry(200, Level::INFO, "smearor_mcp_server::handler", "b"));
        buffer.push(make_entry(300, Level::INFO, "other_crate", "c"));

        let results = buffer.query(Some(Level::TRACE), Some("smearor"), None, None);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].message, "a");
        assert_eq!(results[1].message, "b");

        let results = buffer.query(Some(Level::TRACE), Some("smearor_voice_assistant"), None, None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].message, "a");
    }

    #[test]
    fn test_since_ms_filter() {
        let buffer = LogBuffer::new(100);
        buffer.push(make_entry(1000, Level::INFO, "mod", "old"));
        buffer.push(make_entry(2000, Level::INFO, "mod", "mid"));
        buffer.push(make_entry(3000, Level::INFO, "mod", "new"));

        let results = buffer.query(Some(Level::TRACE), None, Some(2000), None);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].message, "mid");
        assert_eq!(results[1].message, "new");
    }

    #[test]
    fn test_limit_returns_most_recent_n() {
        let buffer = LogBuffer::new(100);
        for i in 0..10 {
            buffer.push(make_entry(i * 100, Level::INFO, "mod", &format!("entry_{i}")));
        }

        let results = buffer.query(Some(Level::TRACE), None, None, Some(3));
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].message, "entry_7");
        assert_eq!(results[1].message, "entry_8");
        assert_eq!(results[2].message, "entry_9");
    }

    #[test]
    fn test_ring_buffer_eviction_at_capacity() {
        let buffer = LogBuffer::new(3);
        buffer.push(make_entry(100, Level::INFO, "mod", "first"));
        buffer.push(make_entry(200, Level::INFO, "mod", "second"));
        buffer.push(make_entry(300, Level::INFO, "mod", "third"));
        assert_eq!(buffer.len(), 3);

        buffer.push(make_entry(400, Level::INFO, "mod", "fourth"));
        assert_eq!(buffer.len(), 3);

        let results = buffer.query(Some(Level::TRACE), None, None, None);
        assert_eq!(results[0].message, "second");
        assert_eq!(results[2].message, "fourth");
    }

    #[test]
    fn test_clear() {
        let buffer = LogBuffer::new(100);
        buffer.push(make_entry(100, Level::INFO, "mod", "a"));
        buffer.push(make_entry(200, Level::INFO, "mod", "b"));
        assert_eq!(buffer.len(), 2);

        buffer.clear();
        assert_eq!(buffer.len(), 0);

        let results = buffer.query(None, None, None, None);
        assert!(results.is_empty());
    }

    #[test]
    fn test_combined_filters() {
        let buffer = LogBuffer::new(100);
        buffer.push(make_entry(1000, Level::DEBUG, "smearor_voice_assistant::react", "debug_old"));
        buffer.push(make_entry(2000, Level::WARN, "smearor_voice_assistant::react", "warn_new"));
        buffer.push(make_entry(3000, Level::ERROR, "smearor_mcp_server", "error_new"));
        buffer.push(make_entry(4000, Level::INFO, "other", "info_new"));

        let results = buffer.query(Some(Level::WARN), Some("smearor_voice_assistant"), Some(1500), Some(10));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].message, "warn_new");
    }

    #[test]
    fn test_empty_buffer_query() {
        let buffer = LogBuffer::new(100);
        let results = buffer.query(Some(Level::TRACE), None, None, None);
        assert!(results.is_empty());
    }

    #[test]
    fn test_query_preserves_chronological_order() {
        let buffer = LogBuffer::new(100);
        buffer.push(make_entry(100, Level::INFO, "mod", "first"));
        buffer.push(make_entry(200, Level::INFO, "mod", "second"));
        buffer.push(make_entry(300, Level::INFO, "mod", "third"));

        let results = buffer.query(None, None, None, None);
        assert_eq!(results[0].message, "first");
        assert_eq!(results[1].message, "second");
        assert_eq!(results[2].message, "third");
    }
}
