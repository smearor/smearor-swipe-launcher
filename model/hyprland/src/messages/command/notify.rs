use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::kill::TOPIC_CTL;
use crate::HyprlandColor;
use crate::HyprlandNotifyIcon;

/// Creates a notification with Hyprland.
#[derive(Clone, Debug, Default)]
pub struct NotifyCommandMessage {
    /// The icon to display with the notification.
    pub icon: HyprlandNotifyIcon,
    /// The duration of the notification in milliseconds.
    pub time_ms: u32,
    /// The color of the notification.
    pub color: HyprlandColor,
    /// The notification message text.
    pub message: String,
}

/// ABI-stable version of `NotifyCommandMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct NotifyCommandMessageStabby {
    /// The icon to display with the notification.
    pub icon: HyprlandNotifyIcon,
    /// The duration of the notification in milliseconds.
    pub time_ms: u32,
    /// The color of the notification.
    pub color: HyprlandColor,
    /// The notification message text.
    pub message: stabby::string::String,
}

impl From<NotifyCommandMessage> for NotifyCommandMessageStabby {
    fn from(value: NotifyCommandMessage) -> Self {
        Self {
            icon: value.icon,
            time_ms: value.time_ms,
            color: value.color,
            message: value.message.into(),
        }
    }
}

impl From<NotifyCommandMessageStabby> for NotifyCommandMessage {
    fn from(value: NotifyCommandMessageStabby) -> Self {
        Self {
            icon: value.icon,
            time_ms: value.time_ms,
            color: value.color,
            message: value.message.to_string(),
        }
    }
}

impl TypedMessage for NotifyCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::NotifyCommandMessage");
}

impl TypedMessage for NotifyCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::NotifyCommandMessageStabby");
}

impl MessageTopic for NotifyCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl MessageTopic for NotifyCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for NotifyCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
