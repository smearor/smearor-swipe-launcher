use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_STATUS: &str = "service.notifications.status";

/// Urgency level of a notification.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq)]
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
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, PartialEq)]
pub struct NotificationAction {
    /// Machine-readable action identifier
    pub key: stabby::string::String,
    /// Human-readable action label
    pub label: stabby::string::String,
}

/// Information about an active notification.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, PartialEq)]
pub struct NotificationInfo {
    /// Unique notification ID
    pub id: u32,
    /// Name of the application that sent the notification
    pub app_name: stabby::string::String,
    /// Notification summary/title
    pub summary: stabby::string::String,
    /// Notification body text
    pub body: stabby::string::String,
    /// Icon name or path
    pub icon: stabby::option::Option<stabby::string::String>,
    /// Urgency level
    pub urgency: UrgencyLevel,
    /// Available action buttons
    pub actions: stabby::vec::Vec<NotificationAction>,
    /// Timestamp when the notification was received (Unix epoch millis)
    pub timestamp: u64,
    /// Timeout in milliseconds (0 = no timeout, -1 = server default)
    pub timeout_ms: i32,
}

/// Status message broadcast by the notifications service to all widgets.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, PartialEq)]
pub struct NotificationStatusMessage {
    /// Whether Do Not Disturb mode is active
    pub do_not_disturb: bool,
    /// List of active notifications
    pub notifications: stabby::vec::Vec<NotificationInfo>,
    /// Number of unread notifications
    pub unread_count: u32,
}

impl NotificationStatusMessage {
    pub fn new(do_not_disturb: bool, notifications: stabby::vec::Vec<NotificationInfo>, unread_count: u32) -> Self {
        Self {
            do_not_disturb,
            notifications,
            unread_count,
        }
    }
}

impl TypedMessage for NotificationStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_notifications_model::NotificationStatusMessage");
}

impl MessageTopic for NotificationStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl SharedMessage for NotificationStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_STATUS
    }
}
