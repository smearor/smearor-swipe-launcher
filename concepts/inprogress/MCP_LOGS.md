# Concept: MCP Logs Tool — Tracing Log Access via MCP

Adds a core MCP tool `launcher_get_logs` that exposes the launcher's tracing log ring buffer to MCP clients. Enables the Automatic Evaluation Skill and other
agents to retrieve diagnostic logs after test runs.

---

## 1. Motivation

- **No programmatic log access**: Tracing logs are only written to stdout. MCP clients cannot retrieve logs to diagnose failures.
- **Evaluation blind spot**: Training traces show ReAct steps but not surrounding context — tool selection rankings, parse errors, timing. These are logged at
  `debug`/`trace` level but invisible to the evaluation skill.
- **No filtering**: `RUST_LOG` controls stdout output but cannot query a specific time window, level, or target after the fact.

---

## 2. Architecture

Core MCP server feature — no model/service/widget crates. Touches two crates:

| Crate                    | Path                      | Responsibility                                                                                  |
|--------------------------|---------------------------|-------------------------------------------------------------------------------------------------|
| `mcp-server`             | `mcp-server/`             | `LogBuffer`, `LogBufferLayer` (tracing Layer), `GetLogsParams`, tool definition, direct handler |
| `smearor-swipe-launcher` | `smearor-swipe-launcher/` | Install `LogBufferLayer` in `init_tracing.rs`, pass `LogBuffer` to `McpServerState`             |

### Data Flow

```
tracing events → registry()
  ├── fmt::Layer → stdout (existing)
  └── LogBufferLayer → LogBuffer (Arc<parking_lot::Mutex<VecDeque<LogEntry>>>)
                              │
                              ▼
                    McpServerState.log_buffer
                              │
                              ▼
                    launcher_get_logs tool (direct query)
```

### Design Decisions

- **Tool, not Resource**: Filter parameters (`level`, `target_prefix`, `since_seconds`, `limit`) allow targeted queries. A resource would return the full buffer
  every time.
- **Ring buffer**: Bounded `VecDeque` (default 10,000 entries) prevents unbounded memory growth. Old entries evicted automatically.
- **Additive layer**: Installed alongside existing `fmt` layer. stdout output unchanged.
- **Direct handler**: The `LogBuffer` is shared via `Arc` and already in `McpServerState`. The tool queries it directly — no command channel round-trip needed.
- **`parking_lot::Mutex` over `std::sync::Mutex`**: The tracing hot path calls `on_event()` for every log event. Under high throughput (`RUST_LOG=trace`),
  `std::sync::Mutex`
  becomes a bottleneck — it blocks all threads logging concurrently and carries poisoning overhead. `parking_lot::Mutex` uses a faster lock implementation
  (futex-based on Linux), never poisons, and performs significantly better under contention. The lock is held only for the `push_back`/`pop_front` pair — the
  `LogEntry` is constructed on the stack before acquiring the lock.

---

## 3. Data Structures

### 3.1 LogEntry

```rust
/// A single tracing log event captured in the ring buffer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    /// Timestamp of the event (Unix epoch milliseconds).
    pub timestamp_ms: u64,
    /// Log level, stored as `tracing::Level` for correct ordinal comparison.
    /// Serialized as string ("trace", "debug", "info", "warn", "error").
    pub level: tracing::Level,
    /// Tracing target (module path, e.g. "smearor_voice_assistant::service").
    pub target: String,
    /// Formatted message text (the `message` field from tracing macros).
    pub message: String,
    /// Additional structured fields from the tracing event (e.g. `user_id = 42`).
    /// Each entry is formatted as `key=value`.
    pub fields: Vec<String>,
    /// Optional file name where the event was recorded.
    pub file: Option<String>,
    /// Optional line number where the event was recorded.
    pub line: Option<u32>,
}
```

### 3.2 LogBuffer

Thread-safe ring buffer. `push()` evicts oldest entry at capacity. `query()` filters by level, target prefix, timestamp, and limit. `len()` returns current
count.

```rust
pub struct LogBuffer {
    inner: parking_lot::Mutex<VecDeque<LogEntry>>,
    capacity: usize,
}
```

Uses `parking_lot::Mutex` instead of `std::sync::Mutex` for performance in the tracing hot path. `parking_lot::Mutex` is faster under contention and never
poisons.

Key methods:

- `new(capacity: usize)` — create with bounded capacity
- `push(entry: LogEntry)` — evict oldest if at capacity
- `query(min_level: Option<tracing::Level>, target_prefix, since_ms, limit) -> Vec<LogEntry>` — filtered retrieval. Uses `tracing::Level` ordinal comparison
  (`Level::ERROR` < `Level::WARN` < `Level::INFO` < `Level::DEBUG` < `Level::TRACE`) — no string comparison. Filter direction: `entry.level <= min_level` (e.g.
  `min_level = INFO` returns `ERROR`, `WARN`, `INFO` — higher or equal priority, not higher verbosity). Iterates **backwards** (`rev()`) and takes the most
  recent N matching entries, then reverses the result to restore chronological order. This ensures `limit` returns the newest entries, not the oldest.
- `clear()` — empty the buffer
- `len() -> usize` — current entry count
- `capacity() -> usize` — maximum entry count

### 3.3 LogBufferLayer

Implements `tracing_subscriber::Layer`. `on_event` constructs the `LogEntry` fully on the stack (metadata extraction, visitor, timestamp), then acquires the
`parking_lot::Mutex` lock only for the brief `push_back`/`pop_front` operation. This minimizes lock duration in the tracing hot path.

Uses a `LogEntryVisitor` implementing `tracing::field::Visit` to extract the formatted message and structured fields from event fields.

The visitor distinguishes between the `message` field (tracing's standard message) and additional structured fields:

```rust
#[derive(Default)]
struct LogEntryVisitor {
    message: String,
    fields: Vec<String>,
}

impl tracing::field::Visit for LogEntryVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields.push(format!("{}={}", field.name(), value));
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        } else {
            self.fields.push(format!("{}={:?}", field.name(), value));
        }
    }
}
```

For `tracing::info!(user_id = 42, "User connected")`, the visitor produces `message = "User connected"` and `fields = ["user_id=42"]`.

---

## 4. MCP Tool: `launcher_get_logs`

### Input Schema

| Parameter       | Type      | Default   | Description                                                                                                      |
|-----------------|-----------|-----------|------------------------------------------------------------------------------------------------------------------|
| `min_level`     | `String`  | `"debug"` | Minimum log level: `trace`, `debug`, `info`, `warn`, `error`. Parsed to `tracing::Level` for ordinal comparison. |
| `target_prefix` | `String?` | none      | Filter by tracing target prefix (e.g. `"smearor_voice_assistant"`)                                               |
| `since_seconds` | `u64?`    | none      | Only entries from last N seconds                                                                                 |
| `limit`         | `usize`   | `200`     | Max entries to return (most recent N)                                                                            |

### Response Format

```json
{
  "entries": [
    {
      "timestamp_ms": 1723124123456,
      "level": "debug",
      "target": "smearor_voice_assistant::react",
      "message": "ReAct iteration 2: action=tool:weather_get_forecast",
      "file": "services/voice_assistant/src/react.rs",
      "line": 342
    }
  ],
  "total_returned": 1,
  "total_in_buffer": 5432,
  "buffer_capacity": 10000
}
```

### GetLogsParams

Defined in `mcp-server/src/command/get_logs.rs`. Implements `McpCommandVariant` and `ToolDefinitionCreator`. Uses `TypedBuilder` and `JsonSchema` derive,
following the same pattern as `SendMessageParams`, `LoadInstanceParams`, etc.

`min_level` is accepted as `String` in the JSON schema (for LLM-friendly enum values) but parsed to `tracing::Level` via `FromStr` in the handler before
querying the buffer. This avoids error-prone string comparison — `tracing::Level` implements `Ord` with `ERROR < WARN < INFO < DEBUG < TRACE`.

```rust
/// Parameters for retrieving launcher logs via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct GetLogsParams {
    /// Minimum log level: "trace", "debug", "info", "warn", "error".
    /// Parsed to `tracing::Level` in the handler for ordinal comparison.
    #[serde(default = "default_min_level")]
    #[builder(default = "debug".to_string())]
    pub min_level: String,
    /// Filter by tracing target prefix (e.g. "smearor_voice_assistant").
    #[serde(default)]
    #[builder(default)]
    pub target_prefix: Option<String>,
    /// Only return entries from the last N seconds.
    #[serde(default)]
    #[builder(default)]
    pub since_seconds: Option<u64>,
    /// Maximum number of entries to return (most recent N). Default: 200.
    #[serde(default = "default_limit")]
    #[builder(default = 200)]
    pub limit: usize,
}
```

In the handler, `min_level` is parsed before querying:

```rust
let min_level = log_params.min_level.parse::<tracing::Level>().map_err( | e| {
format ! ("Invalid min_level '{}': valid values are trace, debug, info, warn, error", log_params.min_level)
});
```

If parsing fails, a `CallToolResult` with `is_error: true` is returned.

---

## 5. Implementation

### 5.1 New Module: `mcp-server/src/log_buffer.rs`

Contains `LogEntry`, `LogBuffer`, `LogBufferLayer`, `LogEntryVisitor`. Exported from `mcp-server/src/lib.rs`.

### 5.2 New Command: `mcp-server/src/command/get_logs.rs`

`GetLogsParams` struct with `McpCommandVariant` impl mapping to `McpCommand::GetLogs`. `ToolDefinitionCreator` impl with `tool_name() = "launcher_get_logs"`.

### 5.3 McpCommand Extension

Add `GetLogs(CommandResponseWrapper<GetLogsParams>)` variant to `McpCommand` enum.

### 5.4 McpServerState Extension

Add `log_buffer: Arc<LogBuffer>` field to `McpServerState`. Required (not default) — passed from `main.rs`.

### 5.5 Tool Registration

Add `GetLogsParams::create_tool_definition()` to `ToolDefinition::core_tools()`.

### 5.6 Direct Handler in `handle_call_tool_request`

In `mcp-server/src/server/handler.rs`, before the generic command dispatch, check for `launcher_get_logs` and query the buffer directly:

```rust
if params.name == "launcher_get_logs" {
let args = params.arguments.unwrap_or_default();
let log_params: GetLogsParams = match serde_json::from_value(args) {
Ok(params) => params,
Err(parse_error) => {
return Ok(CallToolResult {
content: vec ! [TextContent::new(format! (
"Invalid arguments for launcher_get_logs: {parse_error}"
))],
is_error: true,
..Default::default()
});
}
};
let since_ms = log_params.since_seconds.map(| s | {
let now = SystemTime::now().duration_since(UNIX_EPOCH)
.map( | d | d.as_millis() as u64).unwrap_or(0);
now.saturating_sub(s * 1000)
});
let min_level = match log_params.min_level.parse::< tracing::Level > () {
Ok(level) => level,
Err(_) => {
return Ok(CallToolResult {
content: vec ! [TextContent::new(format! (
"Invalid min_level '{}': valid values are trace, debug, info, warn, error",
log_params.min_level
))],
is_error: true,
..Default::default()
});
}
};
let entries = self.state.log_buffer.query(
Some(min_level),
log_params.target_prefix.as_deref(),
since_ms,
Some(log_params.limit),
);
let response = serde_json::json ! ({
"entries": entries,
"total_returned": entries.len(),
"total_in_buffer": self.state.log_buffer.len(),
"buffer_capacity": self.state.log_buffer.capacity(),
});
return Ok(CallToolResult {
content: vec ! [TextContent::new(response.to_string())],
..Default::default()
});
}
```

### 5.7 Tracing Init

`smearor-swipe-launcher/src/init_tracing.rs` changes from `FmtSubscriber` to `tracing_subscriber::registry()` with `fmt::Layer` + `LogBufferLayer`. Returns
`Arc<LogBuffer>`.

### 5.8 Main Integration

`main.rs` captures the `LogBuffer` from `init()` and passes it to `start_mcp_server`, which passes it to `McpServerState`.

---

## 6. Config Integration

### 6.1 Log Buffer Capacity (Optional)

```toml
[services.mcp]
log_buffer_capacity = 10000
```

Optional — default 10,000 is sufficient for evaluation.

### 6.2 RUST_LOG

Existing `RUST_LOG` env var controls which events reach the subscriber. The `LogBufferLayer` captures all events that pass the `EnvFilter`. For full trace
capture, set `RUST_LOG=trace`. For production, `RUST_LOG=debug` or `info`.

---

## 7. Implementation Phases

### Phase 1: LogBuffer and Layer

- Implement `LogEntry`, `LogBuffer`, `LogBufferLayer`, `LogEntryVisitor` in `mcp-server/src/log_buffer.rs`
- Add `tracing = { workspace = true, features = ["serde"] }` to `mcp-server/Cargo.toml`
- Add `parking_lot` to workspace `Cargo.toml` and `mcp-server/Cargo.toml`
- Export from `lib.rs`
- **Exit Criteria**: `cargo build -p smearor-mcp-server` succeeds

### Phase 2: MCP Tool Integration

- Add `GetLogsParams` in `mcp-server/src/command/get_logs.rs`
- Add `McpCommand::GetLogs` variant
- Add `log_buffer` field to `McpServerState`
- Add `GetLogsParams::create_tool_definition()` to `core_tools()`
- Add direct handler in `handle_call_tool_request`
- **Exit Criteria**: `cargo build -p smearor-mcp-server` succeeds, tool appears in `tools/list`

### Phase 3: Launcher Integration

- Modify `init_tracing.rs` to return `Arc<LogBuffer>`
- Pass `LogBuffer` through `start_mcp_server` to `McpServerState`
- **Exit Criteria**: `cargo build` succeeds, launcher starts, logs are captured in buffer

### Phase 4: Verification

- Call `launcher_get_logs` via MCP client
- Verify filters work (level, target, time, limit)
- Verify ring buffer eviction at capacity
- Verify stdout output unchanged
- **Exit Criteria**: All verification tasks pass

---

## 8. Dependencies

| Crate                    | New Dependencies                                                                                                                         |
|--------------------------|------------------------------------------------------------------------------------------------------------------------------------------|
| `mcp-server`             | `tracing` (workspace, **with `serde` feature**), `tracing-subscriber` (workspace), `serde`, `serde_json` (existing), `parking_lot` (new) |
| `smearor-swipe-launcher` | none new (uses `mcp-server` re-exports)                                                                                                  |

`tracing-subscriber` is already a workspace dependency with `env-filter` feature. The `Layer` trait is in the default feature set.
`parking_lot` must be added to the workspace `Cargo.toml` and the `mcp-server` crate's `Cargo.toml`.

**Critical**: `tracing::Level` only implements `Serialize` and `Deserialize` when the `serde` feature is enabled on the `tracing` crate. The workspace
`Cargo.toml` currently defines `tracing = "0.1"` without features. The `mcp-server` crate must override this:

```toml
# mcp-server/Cargo.toml
tracing = { workspace = true, features = ["serde"] }
```

This enables `tracing::Level` to derive `Serialize`/`Deserialize`, which `LogEntry` requires. Without this feature, the build will fail with
`the trait bound `tracing::Level: Serialize` is not satisfied`.

---

## 9. Testing Checklist

- [ ] `launcher_get_logs` returns entries after launcher start
- [ ] `min_level` filter correctly excludes lower levels
- [ ] `target_prefix` filter matches module paths
- [ ] `since_seconds` filter returns only recent entries
- [ ] `limit` parameter returns most recent N entries
- [ ] Ring buffer evicts oldest entries at capacity
- [ ] `total_in_buffer` and `buffer_capacity` are accurate
- [ ] stdout output is unchanged with `LogBufferLayer` installed
- [ ] `RUST_LOG=trace` captures trace-level events in buffer
- [ ] Tool appears in MCP `tools/list` response
- [ ] No `unwrap()` or `expect()` in production code paths
- [ ] Lock duration in `on_event` is minimal (LogEntry built on stack, lock only for push)

---

## 10. Common Pitfalls

- **Lock contention in hot path**: `on_event()` is called for every tracing event. The `LogEntry` must be fully constructed on the stack before acquiring the
  `parking_lot::Mutex` — the lock is held only for the `push_back`/`pop_front` pair. This keeps critical sections to microseconds.
- **No mutex poisoning**: `parking_lot::Mutex` never poisons, so `lock()` always succeeds. This eliminates the `PoisonError` handling needed with
  `std::sync::Mutex`.
- **Message formatting**: The `LogEntryVisitor` must handle both `record_str` and `record_debug` calls. The `message` field from tracing's standard macros uses
  `record_str` with field name `"message"`.
- **Timestamp accuracy**: Use `SystemTime::now()` at event capture time, not at query time. This ensures `since_seconds` filtering is accurate.
- **Buffer capacity vs memory**: Each `LogEntry` contains two `String`s (message, target) plus optional strings (file). At 10,000 entries with average 200-byte
  messages, the buffer uses ~2MB — acceptable.
- **Thread safety**: `LogBufferLayer` must be `Send + Sync`. `Arc<LogBuffer>` with `parking_lot::Mutex<VecDeque>` satisfies this.

---

## 11. Future Enhancements

- **Log level configuration via MCP**: A `launcher_set_log_level` tool to change `RUST_LOG` at runtime without restart.
- **Structured field extraction**: Capture individual tracing fields (not just formatted message) for structured querying.
- **Log subscription**: MCP resource subscription for real-time log streaming instead of polling.
- **Per-plugin log isolation**: Separate buffers per plugin for targeted debugging.
- **Log export**: Tool to export buffer as a file (JSONL or plain text) for offline analysis.
