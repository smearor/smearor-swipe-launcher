use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;

pub const TOPIC_COMMAND: &str = "service.notifications.command";

/// Actions that can be sent from the widget to the notifications service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum NotificationCommandAction {
    #[default]
    /// Dismiss a single notification by ID
    Dismiss,
    /// Dismiss all visible notifications
    DismissAll,
    /// Dismiss the most recent notification
    DismissLast,
    /// Invoke an action button on a notification
    InvokeAction,
    /// Toggle Do Not Disturb mode
    ToggleDoNotDisturb,
}

/// Command message sent from the notifications widget to the notifications service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct NotificationCommandMessage {
    /// The action to execute
    pub action: NotificationCommandAction,
    /// Optional notification ID to target a specific notification
    pub notification_id: Option<u32>,
    /// Optional action key to invoke on a notification
    pub action_key: Option<String>,
}

impl NotificationCommandMessage {
    pub fn new(action: NotificationCommandAction, notification_id: Option<u32>, action_key: Option<String>) -> Self {
        Self {
            action,
            notification_id,
            action_key,
        }
    }

    pub fn dismiss() -> Self {
        Self::new(NotificationCommandAction::Dismiss, None, None)
    }

    pub fn dismiss_all() -> Self {
        Self::new(NotificationCommandAction::DismissAll, None, None)
    }

    pub fn dismiss_last() -> Self {
        Self::new(NotificationCommandAction::DismissLast, None, None)
    }

    pub fn invoke_action(notification_id: u32, action_key: String) -> Self {
        Self::new(NotificationCommandAction::InvokeAction, Some(notification_id), Some(action_key))
    }

    pub fn toggle_do_not_disturb() -> Self {
        Self::new(NotificationCommandAction::ToggleDoNotDisturb, None, None)
    }
}

impl MessageTopic for NotificationCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}
