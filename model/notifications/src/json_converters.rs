use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use stabby::option::Option as StabbyOption;
use stabby::vec::Vec as StabbyVec;

use crate::NotificationAction;
use crate::NotificationCommandAction;
use crate::NotificationCommandMessage;
use crate::NotificationInfo;
use crate::NotificationStatusMessage;
use crate::UrgencyLevel;

fn parse_notification_command_action(value: &serde_json::Value) -> NotificationCommandAction {
    match value.as_str() {
        Some("DismissAll") => NotificationCommandAction::DismissAll,
        Some("DismissLast") => NotificationCommandAction::DismissLast,
        Some("InvokeAction") => NotificationCommandAction::InvokeAction,
        Some("ToggleDoNotDisturb") => NotificationCommandAction::ToggleDoNotDisturb,
        _ => NotificationCommandAction::Dismiss,
    }
}

fn parse_urgency_level(value: &serde_json::Value) -> UrgencyLevel {
    match value.as_str() {
        Some("Low") => UrgencyLevel::Low,
        Some("Critical") => UrgencyLevel::Critical,
        _ => UrgencyLevel::Normal,
    }
}

fn parse_notification_action(value: &serde_json::Value) -> NotificationAction {
    NotificationAction {
        key: value.get("key").and_then(|v| v.as_str()).unwrap_or("").into(),
        label: value.get("label").and_then(|v| v.as_str()).unwrap_or("").into(),
    }
}

fn parse_notification_actions(value: &serde_json::Value) -> StabbyVec<NotificationAction> {
    let mut actions = StabbyVec::new();
    if let Some(arr) = value.as_array() {
        for item in arr {
            actions.push(parse_notification_action(item));
        }
    }
    actions
}

fn parse_notification_info(value: &serde_json::Value) -> NotificationInfo {
    NotificationInfo {
        id: value.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        app_name: value.get("app_name").and_then(|v| v.as_str()).unwrap_or("").into(),
        summary: value.get("summary").and_then(|v| v.as_str()).unwrap_or("").into(),
        body: value.get("body").and_then(|v| v.as_str()).unwrap_or("").into(),
        icon: value
            .get("icon")
            .and_then(|v| if v.is_null() { None } else { Some(v.as_str().unwrap_or("").into()) })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None()),
        urgency: parse_urgency_level(value.get("urgency").unwrap_or(&serde_json::Value::Null)),
        actions: parse_notification_actions(value.get("actions").unwrap_or(&serde_json::Value::Null)),
        timestamp: value.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0),
        timeout_ms: value.get("timeout_ms").and_then(|v| v.as_i64()).unwrap_or(-1) as i32,
    }
}

fn parse_notifications(value: &serde_json::Value) -> StabbyVec<NotificationInfo> {
    let mut notifications = StabbyVec::new();
    if let Some(arr) = value.as_array() {
        for item in arr {
            notifications.push(parse_notification_info(item));
        }
    }
    notifications
}

smearor_swipe_launcher_plugin_api::impl_json_convertible!(NotificationCommandMessageConverter, NotificationCommandMessage, |json: serde_json::Value| {
    let action = parse_notification_command_action(json.get("action").unwrap_or(&serde_json::Value::Null));
    let notification_id = json
        .get("notification_id")
        .and_then(|v| if v.is_null() { None } else { Some(v.as_u64().unwrap_or(0) as u32) })
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None());
    let action_key = json
        .get("action_key")
        .and_then(|v| if v.is_null() { None } else { Some(v.as_str().unwrap_or("").into()) })
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None());
    NotificationCommandMessage::new(action, notification_id, action_key)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(NotificationStatusMessageConverter, NotificationStatusMessage, |json: serde_json::Value| {
    let do_not_disturb = json.get("do_not_disturb").and_then(|v| v.as_bool()).unwrap_or(false);
    let notifications = parse_notifications(json.get("notifications").unwrap_or(&serde_json::Value::Null));
    let unread_count = json.get("unread_count").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    NotificationStatusMessage::new(do_not_disturb, notifications, unread_count)
});

/// Register all JSON converter implementations for notifications messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    NotificationCommandMessageConverter::register_in_host(context);
    NotificationStatusMessageConverter::register_in_host(context);
}
