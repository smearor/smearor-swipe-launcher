use serde::Deserialize;
use serde::Serialize;

/// Single notification entry in the MCP history resource response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationHistoryEntry {
    /// Unique notification ID
    pub id: u32,
    /// Name of the application that sent the notification
    pub app_name: String,
    /// Notification summary/title
    pub summary: String,
    /// Notification body text
    pub body: String,
    /// Urgency level as string ("Low", "Normal", "Critical")
    pub urgency: String,
    /// Timestamp when the notification was received (Unix epoch millis)
    pub timestamp: u64,
}

/// Response body for the `notifications://history` MCP resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationHistoryResponse {
    /// Whether Do Not Disturb mode is active
    pub do_not_disturb: bool,
    /// Number of unread notifications
    pub unread_count: u32,
    /// List of active notifications
    pub notifications: Vec<NotificationHistoryEntry>,
}

/// Response body for the `notifications://dnd` MCP resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationDndResponse {
    /// Whether Do Not Disturb mode is active
    pub do_not_disturb: bool,
}
