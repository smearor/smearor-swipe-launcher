use serde::Deserialize;
use serde::Serialize;
use tracing::Level;

/// Custom serde module for `tracing::Level` — serializes as string and
/// deserializes via `FromStr`, since `tracing` 0.1 has no `serde` feature.
pub mod level_serde {
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serializer;
    use std::str::FromStr;
    use tracing::Level;

    pub fn serialize<S: Serializer>(level: &Level, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&level.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Level, D::Error> {
        let s = String::deserialize(deserializer)?;
        Level::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// A single tracing log event captured in the ring buffer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    /// Timestamp of the event (Unix epoch milliseconds).
    pub timestamp_ms: u64,
    /// Log level, serialized as string ("trace", "debug", "info", "warn", "error").
    /// Uses custom serde module since `tracing` 0.1 has no `serde` feature.
    #[serde(with = "level_serde")]
    pub level: Level,
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

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::Level;

    #[test]
    fn test_log_entry_serde_round_trip() {
        let entry = LogEntry {
            timestamp_ms: 1723124123456,
            level: Level::DEBUG,
            target: "smearor_voice_assistant::react".to_string(),
            message: "ReAct iteration 2".to_string(),
            fields: vec!["user_id=42".to_string()],
            file: Some("src/react.rs".to_string()),
            line: Some(123),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: LogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.timestamp_ms, entry.timestamp_ms);
        assert_eq!(deserialized.level, entry.level);
        assert_eq!(deserialized.target, entry.target);
        assert_eq!(deserialized.message, entry.message);
        assert_eq!(deserialized.fields, entry.fields);
        assert_eq!(deserialized.file, entry.file);
        assert_eq!(deserialized.line, entry.line);
    }

    #[test]
    fn test_log_entry_json_level_serialized_as_string() {
        let entry = LogEntry {
            timestamp_ms: 100,
            level: Level::INFO,
            target: "test".to_string(),
            message: "hello".to_string(),
            fields: Vec::new(),
            file: None,
            line: None,
        };

        let json = serde_json::to_value(&entry).unwrap();
        let obj = json.as_object().unwrap();
        assert!(obj.contains_key("timestamp_ms"));
        assert!(obj.contains_key("level"));
        assert!(obj.contains_key("target"));
        assert!(obj.contains_key("message"));
        assert!(obj.contains_key("fields"));
        assert!(obj.contains_key("file"));
        assert!(obj.contains_key("line"));
        assert_eq!(obj["level"].as_str().unwrap(), "INFO");
    }

    #[test]
    fn test_log_entry_level_deserialize_from_string() {
        let json = r#"{
            "timestamp_ms": 100,
            "level": "WARN",
            "target": "test",
            "message": "warning",
            "fields": [],
            "file": null,
            "line": null
        }"#;
        let entry: LogEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.level, Level::WARN);
    }

    #[test]
    fn test_log_entry_level_deserialize_lowercase() {
        let json = r#"{
            "timestamp_ms": 100,
            "level": "error",
            "target": "test",
            "message": "error",
            "fields": [],
            "file": null,
            "line": null
        }"#;
        let entry: LogEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.level, Level::ERROR);
    }

    #[test]
    fn test_log_entry_level_deserialize_invalid() {
        let json = r#"{
            "timestamp_ms": 100,
            "level": "verbose",
            "target": "test",
            "message": "msg",
            "fields": [],
            "file": null,
            "line": null
        }"#;
        let result: Result<LogEntry, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
