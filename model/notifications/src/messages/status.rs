use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;

pub const TOPIC_STATUS: &str = "service.notifications.status";

/// Urgency level of a notification.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub enum UrgencyLevel {
    /// Low urgency (e.g. chat messages, social media)
    Low,
    /// Normal urgency (default)
    #[default]
    Normal,
    /// Critical urgency (e.g. system warnings, low battery)
    Critical,
}

/// An action button exposed by a notification.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NotificationAction {
    /// Machine-readable action identifier
    pub key: String,
    /// Human-readable action label
    pub label: String,
}

/// Information about an active notification.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NotificationInfo {
    /// Unique notification ID
    pub id: u32,
    /// Name of the application that sent the notification
    pub app_name: String,
    /// Notification summary/title
    pub summary: String,
    /// Notification body text
    pub body: String,
    /// Icon name or path
    pub icon: Option<String>,
    /// Urgency level
    pub urgency: UrgencyLevel,
    /// Available action buttons
    pub actions: Vec<NotificationAction>,
    /// Timestamp when the notification was received (Unix epoch millis)
    pub timestamp: u64,
    /// Timeout in milliseconds (0 = no timeout, -1 = server default)
    pub timeout_ms: i32,
}

/// Status message broadcast by the notifications service to all widgets.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NotificationStatusMessage {
    /// Whether Do Not Disturb mode is active
    pub do_not_disturb: bool,
    /// List of active notifications
    pub notifications: Vec<NotificationInfo>,
    /// Number of unread notifications
    pub unread_count: u32,
}

impl NotificationStatusMessage {
    pub fn new(do_not_disturb: bool, notifications: Vec<NotificationInfo>, unread_count: u32) -> Self {
        Self {
            do_not_disturb,
            notifications,
            unread_count,
        }
    }
}

impl MessageTopic for NotificationStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}
