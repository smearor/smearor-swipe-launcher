use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_COMMAND: &str = "service.notifications.command";

/// Actions that can be sent from the widget to the notifications service.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
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
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct NotificationCommandMessage {
    /// The action to execute
    pub action: NotificationCommandAction,
    /// Optional notification ID to target a specific notification
    pub notification_id: stabby::option::Option<u32>,
    /// Optional action key to invoke on a notification
    pub action_key: stabby::option::Option<stabby::string::String>,
}

impl NotificationCommandMessage {
    pub fn new(
        action: NotificationCommandAction,
        notification_id: stabby::option::Option<u32>,
        action_key: stabby::option::Option<stabby::string::String>,
    ) -> Self {
        Self {
            action,
            notification_id,
            action_key,
        }
    }

    pub fn dismiss_id(id: u32) -> Self {
        Self::new(NotificationCommandAction::Dismiss, stabby::option::Option::Some(id), stabby::option::Option::None())
    }

    pub fn dismiss_all() -> Self {
        Self::new(NotificationCommandAction::DismissAll, stabby::option::Option::None(), stabby::option::Option::None())
    }

    pub fn dismiss_last() -> Self {
        Self::new(NotificationCommandAction::DismissLast, stabby::option::Option::None(), stabby::option::Option::None())
    }

    pub fn invoke_action(notification_id: u32, action_key: stabby::string::String) -> Self {
        Self::new(
            NotificationCommandAction::InvokeAction,
            stabby::option::Option::Some(notification_id),
            stabby::option::Option::Some(action_key),
        )
    }

    pub fn toggle_do_not_disturb() -> Self {
        Self::new(NotificationCommandAction::ToggleDoNotDisturb, stabby::option::Option::None(), stabby::option::Option::None())
    }
}

impl TypedMessage for NotificationCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_notifications_model::NotificationCommandMessage");
}

impl MessageTopic for NotificationCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl SharedMessage for NotificationCommandMessage {
    fn topic(&self) -> &'static str {
        TOPIC_COMMAND
    }
}
