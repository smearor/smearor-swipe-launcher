use serde::Deserialize;
use serde::Serialize;

use crate::logs::entry::LogEntry;

/// Response payload for the `launcher_get_logs` MCP tool.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogQueryResponse {
    /// Matching log entries (most recent N, in chronological order).
    pub entries: Vec<LogEntry>,
    /// Number of entries returned in this response.
    pub total_returned: usize,
    /// Total number of entries currently in the buffer.
    pub total_in_buffer: usize,
    /// Maximum number of entries the buffer can hold.
    pub buffer_capacity: usize,
}
