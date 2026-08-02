use serde::Serialize;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use std::time::Instant;
use tracing::debug;

/// Performance metrics for a single operation type.
#[derive(Clone, Debug, Default, Serialize)]
pub struct OperationMetrics {
    /// Total number of invocations.
    pub count: u64,
    /// Total accumulated time in milliseconds.
    pub total_time_ms: u64,
    /// Fastest invocation in milliseconds.
    pub min_time_ms: u64,
    /// Slowest invocation in milliseconds.
    pub max_time_ms: u64,
}

impl OperationMetrics {
    /// Records a single timing sample.
    pub fn record(&mut self, duration_ms: u64) {
        self.count += 1;
        self.total_time_ms += duration_ms;
        if self.min_time_ms == 0 || duration_ms < self.min_time_ms {
            self.min_time_ms = duration_ms;
        }
        if duration_ms > self.max_time_ms {
            self.max_time_ms = duration_ms;
        }
    }

    /// Returns the average invocation time in milliseconds.
    #[must_use]
    pub fn avg_time_ms(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.total_time_ms as f64 / self.count as f64
    }
}

/// Aggregated performance metrics across all operation types.
#[derive(Clone, Debug, Default, Serialize)]
pub struct PerformanceReport {
    /// LLM inference timings.
    pub llm_inference: OperationMetrics,
    /// Tool invocation timings (MCP roundtrip).
    pub tool_invocation: OperationMetrics,
    /// Embedding generation timings.
    pub embedding_generation: OperationMetrics,
    /// Speech recognition (Whisper) timings.
    pub speech_recognition: OperationMetrics,
    /// Tool selection (semantic embedding) timings.
    pub tool_selection: OperationMetrics,
    /// Total ReAct loop durations.
    pub react_loop: OperationMetrics,
    /// Cache hit count for tool results.
    pub tool_cache_hits: u64,
    /// Cache miss count for tool results.
    pub tool_cache_misses: u64,
}

impl PerformanceReport {
    /// Returns the tool cache hit rate as a percentage (0.0–100.0).
    #[must_use]
    pub fn tool_cache_hit_rate(&self) -> f64 {
        let total = self.tool_cache_hits + self.tool_cache_misses;
        if total == 0 {
            return 0.0;
        }
        self.tool_cache_hits as f64 / total as f64 * 100.0
    }

    /// Logs a summary of all collected metrics at debug level.
    pub fn log_summary(&self) {
        debug!(
            "Performance report — \
             LLM: {count} calls, avg {avg:.1}ms | \
             Tools: {tool_count} calls, avg {tool_avg:.1}ms | \
             Embeddings: {emb_count} calls, avg {emb_avg:.1}ms | \
             STT: {stt_count} calls, avg {stt_avg:.1}ms | \
             Tool selection: {sel_count} calls, avg {sel_avg:.2}ms | \
             ReAct loops: {react_count} calls, avg {react_avg:.1}ms | \
             Tool cache: {hits} hits / {misses} misses ({rate:.1}%)",
            count = self.llm_inference.count,
            avg = self.llm_inference.avg_time_ms(),
            tool_count = self.tool_invocation.count,
            tool_avg = self.tool_invocation.avg_time_ms(),
            emb_count = self.embedding_generation.count,
            emb_avg = self.embedding_generation.avg_time_ms(),
            stt_count = self.speech_recognition.count,
            stt_avg = self.speech_recognition.avg_time_ms(),
            sel_count = self.tool_selection.count,
            sel_avg = self.tool_selection.avg_time_ms(),
            react_count = self.react_loop.count,
            react_avg = self.react_loop.avg_time_ms(),
            hits = self.tool_cache_hits,
            misses = self.tool_cache_misses,
            rate = self.tool_cache_hit_rate(),
        );
    }
}

/// Thread-safe performance monitor that collects timing metrics
/// across all voice assistant operations.
#[derive(Clone)]
pub struct PerformanceMonitor {
    report: Arc<RwLock<PerformanceReport>>,
}

impl PerformanceMonitor {
    /// Creates a new empty performance monitor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            report: Arc::new(RwLock::new(PerformanceReport::default())),
        }
    }

    /// Records an LLM inference timing.
    pub fn record_llm_inference(&self, duration: Duration) {
        self.record(|report| &mut report.llm_inference, duration);
    }

    /// Records a tool invocation timing.
    pub fn record_tool_invocation(&self, duration: Duration) {
        self.record(|report| &mut report.tool_invocation, duration);
    }

    /// Records an embedding generation timing.
    pub fn record_embedding(&self, duration: Duration) {
        self.record(|report| &mut report.embedding_generation, duration);
    }

    /// Records a speech recognition timing.
    pub fn record_speech_recognition(&self, duration: Duration) {
        self.record(|report| &mut report.speech_recognition, duration);
    }

    /// Records a tool selection timing.
    pub fn record_tool_selection(&self, duration: Duration) {
        self.record(|report| &mut report.tool_selection, duration);
    }

    /// Records a full ReAct loop timing.
    pub fn record_react_loop(&self, duration: Duration) {
        self.record(|report| &mut report.react_loop, duration);
    }

    /// Records a tool cache hit.
    pub fn record_tool_cache_hit(&self) {
        if let Ok(mut report) = self.report.write() {
            report.tool_cache_hits += 1;
        }
    }

    /// Records a tool cache miss.
    pub fn record_tool_cache_miss(&self) {
        if let Ok(mut report) = self.report.write() {
            report.tool_cache_misses += 1;
        }
    }

    /// Returns a snapshot of the current performance report.
    #[must_use]
    pub fn snapshot(&self) -> PerformanceReport {
        self.report.read().map(|r| r.clone()).unwrap_or_default()
    }

    /// Logs a summary of all collected metrics.
    pub fn log_summary(&self) {
        if let Ok(report) = self.report.read() {
            report.log_summary();
        }
    }

    /// Resets all collected metrics.
    pub fn reset(&self) {
        if let Ok(mut report) = self.report.write() {
            *report = PerformanceReport::default();
        }
    }

    fn record<F>(&self, accessor: F, duration: Duration)
    where
        F: FnOnce(&mut PerformanceReport) -> &mut OperationMetrics,
    {
        if let Ok(mut report) = self.report.write() {
            accessor(&mut report).record(duration.as_millis() as u64);
        }
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// A timing guard that records its elapsed time when dropped.
/// Used for scoped timing of operations.
pub struct TimingGuard {
    monitor: PerformanceMonitor,
    start: Instant,
    recorder: fn(&PerformanceMonitor, Duration),
}

impl TimingGuard {
    /// Starts a new timing guard for the given operation type.
    #[must_use]
    pub fn start(monitor: &PerformanceMonitor, recorder: fn(&PerformanceMonitor, Duration)) -> Self {
        Self {
            monitor: monitor.clone(),
            start: Instant::now(),
            recorder,
        }
    }
}

impl Drop for TimingGuard {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        (self.recorder)(&self.monitor, duration);
    }
}
