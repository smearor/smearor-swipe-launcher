use crate::performance::PerformanceMonitor;
use crate::tool_cache::ToolCache;
use crate::tool_router::ToolRouter;
use smearor_voice_assistant_model::ToolCatalogEntry;
use std::time::Instant;
use tracing::debug;

/// Runs a micro-benchmark for tool selection (semantic embedding).
/// Measures the time to select tools for a set of sample queries.
pub fn benchmark_tool_selection(num_iterations: usize) {
    let mut router = ToolRouter::new();

    let catalog: Vec<ToolCatalogEntry> = vec![
        sample_tool("button_shelly_fan_button", "Toggle the fan on or off"),
        sample_tool("app_launcher_exec", "Launch an application by desktop file"),
        sample_tool("app_launcher_terminate", "Terminate a running application"),
        sample_tool("audio_set_volume", "Set the system audio volume"),
        sample_tool("mpris_play", "Play media on the active player"),
        sample_tool("mpris_pause", "Pause media on the active player"),
        sample_tool("mpris_toggle_play_pause", "Toggle play/pause on the active player"),
        sample_tool("sysinfo_get_cpu_usage", "Get current CPU usage percentage"),
        sample_tool("sysinfo_get_memory_usage", "Get current memory usage"),
        sample_tool("sysinfo_get_temperature", "Get CPU temperature"),
    ];
    router.rebuild(&catalog, None);

    let queries = vec![
        "turn on the fan",
        "launch firefox",
        "set volume to 50",
        "play music",
        "cpu temperature",
        "stop the music",
        "memory usage",
    ];

    let start = Instant::now();
    for _ in 0..num_iterations {
        for query in &queries {
            let _ = router.select_tools(query, 5, 0.3);
        }
    }
    let elapsed = start.elapsed();
    let total_calls = num_iterations * queries.len();
    let avg_us = elapsed.as_micros() as f64 / total_calls as f64;

    debug!("Benchmark: tool_selection — {} calls in {:?} (avg {:.1}µs/call)", total_calls, elapsed, avg_us);
}

/// Runs a micro-benchmark for tool cache operations (insert + get).
pub fn benchmark_tool_cache(num_iterations: usize) {
    let cache = ToolCache::new();
    let args = serde_json::json!({"key": "value", "number": 42});
    let result = smearor_voice_assistant_model::ToolResult::success("test_tool", "result".to_string(), 10);

    let start = Instant::now();
    for i in 0..num_iterations {
        let args_i = serde_json::json!({"index": i, "data": "benchmark"});
        cache.insert("bench_tool", &args_i, result.clone());
    }
    let insert_elapsed = start.elapsed();
    let insert_avg_ns = insert_elapsed.as_nanos() as f64 / num_iterations as f64;

    let start = Instant::now();
    for i in 0..num_iterations {
        let args_i = serde_json::json!({"index": i, "data": "benchmark"});
        let _ = cache.get("bench_tool", &args_i);
    }
    let get_elapsed = start.elapsed();
    let get_avg_ns = get_elapsed.as_nanos() as f64 / num_iterations as f64;

    debug!(
        "Benchmark: tool_cache — {} inserts in {:?} (avg {:.0}ns/insert), {} gets in {:?} (avg {:.0}ns/get)",
        num_iterations, insert_elapsed, insert_avg_ns, num_iterations, get_elapsed, get_avg_ns
    );
}

/// Runs a micro-benchmark for the performance monitor itself.
pub fn benchmark_performance_monitor(num_iterations: usize) {
    let monitor = PerformanceMonitor::new();

    let start = Instant::now();
    for _ in 0..num_iterations {
        monitor.record_llm_inference(std::time::Duration::from_millis(100));
        monitor.record_tool_invocation(std::time::Duration::from_millis(50));
        monitor.record_tool_cache_hit();
        monitor.record_tool_cache_miss();
    }
    let elapsed = start.elapsed();
    let avg_ns = elapsed.as_nanos() as f64 / (num_iterations as f64 * 4.0);

    let report = monitor.snapshot();
    debug!(
        "Benchmark: performance_monitor — {} records in {:?} (avg {:.0}ns/record), snapshot: llm={} tool={} hits={} misses={}",
        num_iterations * 4,
        elapsed,
        avg_ns,
        report.llm_inference.count,
        report.tool_invocation.count,
        report.tool_cache_hits,
        report.tool_cache_misses
    );
}

/// Runs all benchmarks and logs results.
pub fn run_all_benchmarks() {
    debug!("=== Starting benchmark suite ===");
    benchmark_tool_selection(1000);
    benchmark_tool_cache(10_000);
    benchmark_performance_monitor(10_000);
    debug!("=== Benchmark suite complete ===");
}

fn sample_tool(name: &str, description: &str) -> ToolCatalogEntry {
    ToolCatalogEntry {
        name: name.to_string(),
        description: description.to_string(),
        input_schema: "{}".to_string(),
    }
}
