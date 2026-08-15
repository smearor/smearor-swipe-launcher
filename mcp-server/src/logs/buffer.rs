use std::collections::VecDeque;

use parking_lot::Mutex;
use tracing::Level;

use crate::logs::entry::LogEntry;

/// Thread-safe ring buffer for capturing tracing log events.
///
/// Uses per-level ring buffers so that high-volume trace/debug logs cannot
/// evict important error/warn/info entries. Each level gets its own bounded
/// `VecDeque`. `push()` routes into the appropriate level buffer.
/// `query()` merges entries from all levels >= `min_level`.
#[derive(Debug)]
pub struct LogBuffer {
    /// Per-level ring buffers, indexed by `Level` ordering (ERROR=0 .. TRACE=4).
    /// Each buffer has its own capacity, preventing noisy levels from
    /// evicting important entries in other levels.
    buffers: [Mutex<VecDeque<LogEntry>>; 5],
    /// Per-level capacities, indexed identically to `buffers`.
    capacities: [usize; 5],
}

impl LogBuffer {
    /// Create a new `LogBuffer` with the given total capacity distributed
    /// across per-level buffers.
    ///
    /// The capacity is split as follows:
    /// - ERROR: 10% (min 100)
    /// - WARN: 15% (min 200)
    /// - INFO: 25% (min 500)
    /// - DEBUG: 25% (min 500)
    /// - TRACE: 25% (min 500)
    pub fn new(capacity: usize) -> Self {
        let capacities = split_capacity(capacity);
        Self {
            buffers: [
                Mutex::new(VecDeque::with_capacity(capacities[0])),
                Mutex::new(VecDeque::with_capacity(capacities[1])),
                Mutex::new(VecDeque::with_capacity(capacities[2])),
                Mutex::new(VecDeque::with_capacity(capacities[3])),
                Mutex::new(VecDeque::with_capacity(capacities[4])),
            ],
            capacities,
        }
    }

    /// Push a new entry into the ring buffer for its level, evicting the
    /// oldest if that level's buffer is at capacity.
    pub fn push(&self, entry: LogEntry) {
        let index = level_to_index(&entry.level);
        let mut guard = self.buffers[index].lock();
        if guard.len() >= self.capacities[index] {
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
    /// Collects matching entries from all per-level buffers, merges them
    /// chronologically, then returns the most recent `limit` entries.
    pub fn query(&self, min_level: Option<Level>, target_prefix: Option<&str>, since_ms: Option<u64>, limit: Option<usize>) -> Vec<LogEntry> {
        let max = limit.unwrap_or(usize::MAX);
        let max_index = match min_level {
            Some(level) => level_to_index(&level),
            None => 4, // TRACE — all levels
        };

        // Collect matching entries from all relevant level buffers.
        let mut merged: Vec<LogEntry> = Vec::new();
        for index in 0..=max_index {
            let guard = self.buffers[index].lock();
            for entry in guard.iter() {
                if let Some(prefix) = target_prefix {
                    if !entry.target.starts_with(prefix) {
                        continue;
                    }
                }
                if let Some(since) = since_ms {
                    if entry.timestamp_ms < since {
                        continue;
                    }
                }
                merged.push(entry.clone());
            }
        }

        // Sort chronologically by timestamp_ms (stable for equal timestamps).
        merged.sort_by_key(|entry| entry.timestamp_ms);

        // Return the most recent `max` entries.
        if merged.len() > max {
            let drop_count = merged.len() - max;
            merged.drain(..drop_count);
            merged
        } else {
            merged
        }
    }

    /// Clear all entries from all level buffers.
    pub fn clear(&self) {
        for buffer in &self.buffers {
            buffer.lock().clear();
        }
    }

    /// Current total number of entries across all level buffers.
    pub fn len(&self) -> usize {
        self.buffers.iter().map(|buffer| buffer.lock().len()).sum()
    }

    /// Total maximum number of entries across all level buffers.
    pub fn capacity(&self) -> usize {
        self.capacities.iter().sum()
    }

    /// Per-level entry counts, indexed as `[error, warn, info, debug, trace]`.
    pub fn per_level_counts(&self) -> [usize; 5] {
        [
            self.buffers[0].lock().len(),
            self.buffers[1].lock().len(),
            self.buffers[2].lock().len(),
            self.buffers[3].lock().len(),
            self.buffers[4].lock().len(),
        ]
    }

    /// Per-level capacities, indexed as `[error, warn, info, debug, trace]`.
    pub fn per_level_capacities(&self) -> [usize; 5] {
        self.capacities
    }
}

/// Maps a `tracing::Level` to a buffer index (ERROR=0, WARN=1, INFO=2, DEBUG=3, TRACE=4).
///
/// This is intentionally **reversed** from `tracing`'s internal `LevelInner`
/// ordering (Trace=0..Error=4) so that `query()` can iterate `0..=max_index`
/// to collect all levels with priority >= `min_level`.
fn level_to_index(level: &Level) -> usize {
    match level {
        &Level::ERROR => 0,
        &Level::WARN => 1,
        &Level::INFO => 2,
        &Level::DEBUG => 3,
        &Level::TRACE => 4,
    }
}

/// Splits a total capacity across the 5 log levels.
///
/// Returns `[error_cap, warn_cap, info_cap, debug_cap, trace_cap]`.
/// Higher-priority levels get smaller but guaranteed minimums so that
/// trace/debug noise can never fully displace error/warn entries.
fn split_capacity(total: usize) -> [usize; 5] {
    const MIN_ERROR: usize = 100;
    const MIN_WARN: usize = 200;
    const MIN_INFO: usize = 500;
    const MIN_DEBUG: usize = 500;
    const MIN_TRACE: usize = 500;
    const MIN_TOTAL: usize = MIN_ERROR + MIN_WARN + MIN_INFO + MIN_DEBUG + MIN_TRACE;

    if total <= MIN_TOTAL {
        return [MIN_ERROR, MIN_WARN, MIN_INFO, MIN_DEBUG, MIN_TRACE];
    }

    let remaining = total - MIN_TOTAL;
    let extra = remaining / 5;
    let remainder = remaining % 5;

    // Distribute remainder to higher-priority levels first.
    let mut result = [
        MIN_ERROR + extra + (if remainder > 0 { 1 } else { 0 }),
        MIN_WARN + extra + (if remainder > 1 { 1 } else { 0 }),
        MIN_INFO + extra + (if remainder > 2 { 1 } else { 0 }),
        MIN_DEBUG + extra + (if remainder > 3 { 1 } else { 0 }),
        MIN_TRACE + extra,
    ];

    // Ensure the sum matches exactly.
    let sum: usize = result.iter().sum();
    if sum != total {
        result[4] += total - sum;
    }

    result
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
        // Small capacities are clamped to the minimum per-level guarantees.
        let buffer = LogBuffer::new(42);
        assert_eq!(buffer.capacity(), 100 + 200 + 500 + 500 + 500);
    }

    #[test]
    fn test_capacity_large() {
        let buffer = LogBuffer::new(10000);
        assert_eq!(buffer.capacity(), 10000);
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
    fn test_per_level_eviction_does_not_cross_levels() {
        // With a large capacity, INFO buffer holds all entries.
        let buffer = LogBuffer::new(10000);
        // Fill INFO buffer beyond its capacity to trigger eviction.
        let info_cap = split_capacity(10000)[2]; // INFO index = 2
        for i in 0..(info_cap + 5) {
            buffer.push(make_entry(i as u64 * 100, Level::INFO, "mod", &format!("info_{i}")));
        }
        // ERROR buffer should be unaffected by INFO eviction.
        buffer.push(make_entry(0, Level::ERROR, "mod", "error_0"));

        let error_results = buffer.query(Some(Level::ERROR), None, None, None);
        assert_eq!(error_results.len(), 1);
        assert_eq!(error_results[0].message, "error_0");

        // INFO buffer should have evicted the oldest entries.
        let info_results = buffer.query(Some(Level::INFO), None, None, None);
        // Includes 1 ERROR entry + info_cap INFO entries.
        assert_eq!(info_results.len(), info_cap + 1);
        // First 5 INFO entries should have been evicted; first remaining is info_5.
        assert_eq!(info_results[1].message, "info_5");
    }

    #[test]
    fn test_trace_does_not_evict_error() {
        // Verify that flooding TRACE does not evict ERROR entries.
        let buffer = LogBuffer::new(10000);
        buffer.push(make_entry(0, Level::ERROR, "mod", "important_error"));

        // Flood with trace entries.
        let trace_cap = split_capacity(10000)[4]; // TRACE index = 4
        for i in 0..(trace_cap + 100) {
            buffer.push(make_entry((i + 1) as u64, Level::TRACE, "mod", &format!("trace_{i}")));
        }

        let error_results = buffer.query(Some(Level::ERROR), None, None, None);
        assert_eq!(error_results.len(), 1);
        assert_eq!(error_results[0].message, "important_error");
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
