use crate::UrgencyLevel;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `notifications_send` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct NotificationSendArgs {
    /// Notification summary/title
    pub summary: String,
    /// Notification body text
    pub body: String,
    /// Urgency level: "low", "normal", or "critical" (default: "normal")
    pub urgency: Option<String>,
}

impl NotificationSendArgs {
    pub fn urgency_level(&self) -> UrgencyLevel {
        match self.urgency.as_deref() {
            Some("low") => UrgencyLevel::Low,
            Some("critical") => UrgencyLevel::Critical,
            _ => UrgencyLevel::Normal,
        }
    }
}

/// Arguments for the `notifications_toggle_dnd` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct NotificationToggleDndArgs {
    /// Whether to enable or disable Do-Not-Disturb (default: true)
    pub enabled: Option<bool>,
}

/// Arguments for the `notifications_clear` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct NotificationClearArgs {}
