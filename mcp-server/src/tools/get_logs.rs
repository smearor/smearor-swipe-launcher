use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde_json::Value;

use crate::LogBuffer;
use crate::LogQueryResponse;
use crate::McpError;
use crate::command::GetLogsParams;
use crate::tools::ToolResult;

/// Handle the `launcher_get_logs` tool call directly, querying the `LogBuffer`
/// without going through the command channel.
///
/// Returns an error if the log buffer is disabled (`None`).
pub fn handle_get_logs(log_buffer: &Option<Arc<LogBuffer>>, params: Option<&Value>) -> ToolResult {
    let log_buffer = log_buffer.as_ref().ok_or_else(|| {
        McpError::InvalidParams("Log buffer is disabled. Set log_buffer_enabled = true and log_buffer_capacity > 0 in services.toml.".to_string())
    })?;

    let value = params.cloned().unwrap_or(Value::Object(Default::default()));
    let log_params: GetLogsParams = serde_json::from_value(value).map_err(|e| McpError::InvalidParams(e.to_string()))?;

    let min_level = log_params
        .min_level
        .parse::<tracing::Level>()
        .map_err(|_| McpError::InvalidParams(format!("Invalid min_level '{}': valid values are trace, debug, info, warn, error", log_params.min_level)))?;

    let since_ms = log_params.since_seconds.map(|s| {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        now.saturating_sub(s * 1000)
    });

    let entries = log_buffer.query(Some(min_level), log_params.target_prefix.as_deref(), since_ms, Some(log_params.limit));

    let counts = log_buffer.per_level_counts();
    let caps = log_buffer.per_level_capacities();
    let per_level = vec![
        crate::LevelStats {
            count: counts[0],
            capacity: caps[0],
        },
        crate::LevelStats {
            count: counts[1],
            capacity: caps[1],
        },
        crate::LevelStats {
            count: counts[2],
            capacity: caps[2],
        },
        crate::LevelStats {
            count: counts[3],
            capacity: caps[3],
        },
        crate::LevelStats {
            count: counts[4],
            capacity: caps[4],
        },
    ];

    let response = LogQueryResponse {
        total_returned: entries.len(),
        total_in_buffer: log_buffer.len(),
        buffer_capacity: log_buffer.capacity(),
        per_level,
        entries,
    };

    serde_json::to_value(&response).map_err(|e| McpError::InvalidParams(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LogEntry;
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

    fn populated_buffer() -> Option<Arc<LogBuffer>> {
        let buffer = Arc::new(LogBuffer::new(100));
        buffer.push(make_entry(1000, Level::TRACE, "smearor_voice_assistant::react", "trace_msg"));
        buffer.push(make_entry(2000, Level::DEBUG, "smearor_voice_assistant::react", "debug_msg"));
        buffer.push(make_entry(3000, Level::INFO, "smearor_mcp_server::handler", "info_msg"));
        buffer.push(make_entry(4000, Level::WARN, "smearor_voice_assistant::service", "warn_msg"));
        buffer.push(make_entry(5000, Level::ERROR, "other_crate", "error_msg"));
        Some(buffer)
    }

    #[test]
    fn test_handle_get_logs_returns_entries() {
        let buffer = populated_buffer();
        let params = serde_json::json!({});
        let result = handle_get_logs(&buffer, Some(&params));
        assert!(result.is_ok());
        let value = result.unwrap();
        let obj = value.as_object().unwrap();
        assert!(obj.contains_key("entries"));
        assert!(obj.contains_key("total_returned"));
        assert!(obj.contains_key("total_in_buffer"));
        assert!(obj.contains_key("buffer_capacity"));
        assert_eq!(obj["total_in_buffer"].as_u64().unwrap(), 5);
        // Capacity is clamped to minimum per-level guarantees (100+200+500+500+500).
        assert_eq!(obj["buffer_capacity"].as_u64().unwrap(), 1800);
        assert!(obj.contains_key("per_level"));
    }

    #[test]
    fn test_handle_get_logs_min_level_filter() {
        let buffer = populated_buffer();
        let params = serde_json::json!({"min_level": "warn"});
        let result = handle_get_logs(&buffer, Some(&params));
        assert!(result.is_ok());
        let value = result.unwrap();
        let entries = value["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["message"].as_str().unwrap(), "warn_msg");
        assert_eq!(entries[1]["message"].as_str().unwrap(), "error_msg");
    }

    #[test]
    fn test_handle_get_logs_target_prefix_filter() {
        let buffer = populated_buffer();
        let params = serde_json::json!({"target_prefix": "smearor_voice_assistant", "min_level": "trace"});
        let result = handle_get_logs(&buffer, Some(&params));
        assert!(result.is_ok());
        let value = result.unwrap();
        let entries = value["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0]["message"].as_str().unwrap(), "trace_msg");
    }

    #[test]
    fn test_handle_get_logs_limit() {
        let buffer = populated_buffer();
        let params = serde_json::json!({"limit": 2});
        let result = handle_get_logs(&buffer, Some(&params));
        assert!(result.is_ok());
        let value = result.unwrap();
        let entries = value["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["message"].as_str().unwrap(), "warn_msg");
        assert_eq!(entries[1]["message"].as_str().unwrap(), "error_msg");
        assert_eq!(value["total_returned"].as_u64().unwrap(), 2);
    }

    #[test]
    fn test_handle_get_logs_invalid_min_level() {
        let buffer = populated_buffer();
        let params = serde_json::json!({"min_level": "verbose"});
        let result = handle_get_logs(&buffer, Some(&params));
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Invalid min_level"));
        assert!(msg.contains("verbose"));
    }

    #[test]
    fn test_handle_get_logs_default_params() {
        let buffer = populated_buffer();
        let result = handle_get_logs(&buffer, None);
        assert!(result.is_ok());
        let value = result.unwrap();
        let entries = value["entries"].as_array().unwrap();
        // Default min_level is "debug" — filters out TRACE
        assert_eq!(entries.len(), 4);
    }

    #[test]
    fn test_handle_get_logs_empty_buffer() {
        let buffer = Some(Arc::new(LogBuffer::new(100)));
        let params = serde_json::json!({});
        let result = handle_get_logs(&buffer, Some(&params));
        assert!(result.is_ok());
        let value = result.unwrap();
        let entries = value["entries"].as_array().unwrap();
        assert!(entries.is_empty());
        assert_eq!(value["total_returned"].as_u64().unwrap(), 0);
        assert_eq!(value["total_in_buffer"].as_u64().unwrap(), 0);
    }

    #[test]
    fn test_handle_get_logs_disabled_returns_error() {
        let buffer: Option<Arc<LogBuffer>> = None;
        let params = serde_json::json!({});
        let result = handle_get_logs(&buffer, Some(&params));
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Log buffer is disabled"));
    }

    #[test]
    fn test_handle_get_logs_since_seconds() {
        let buffer = Some(Arc::new(LogBuffer::new(100)));
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let buf = buffer.as_ref().unwrap();
        buf.push(make_entry(now - 5000, Level::INFO, "mod", "old_5s"));
        buf.push(make_entry(now - 1000, Level::INFO, "mod", "recent_1s"));
        buf.push(make_entry(now, Level::INFO, "mod", "now"));

        let params = serde_json::json!({"since_seconds": 2});
        let result = handle_get_logs(&buffer, Some(&params));
        assert!(result.is_ok());
        let value = result.unwrap();
        let entries = value["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["message"].as_str().unwrap(), "recent_1s");
        assert_eq!(entries[1]["message"].as_str().unwrap(), "now");
    }

    #[test]
    fn test_handle_get_logs_combined_filters() {
        let buffer = populated_buffer();
        let params = serde_json::json!({
            "min_level": "debug",
            "target_prefix": "smearor_voice_assistant",
            "limit": 1
        });
        let result = handle_get_logs(&buffer, Some(&params));
        assert!(result.is_ok());
        let value = result.unwrap();
        let entries = value["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["message"].as_str().unwrap(), "warn_msg");
    }
}
