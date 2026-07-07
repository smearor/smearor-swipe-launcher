use smearor_swipe_launcher_plugin_api::FfiCoreContext;

use crate::DesktopFileCommandAction;
use crate::DesktopFileCommandMessage;
use crate::DesktopFileCommandMessageStabby;
use crate::DesktopFileStatus;
use crate::DesktopFileStatusMessage;
use crate::DesktopFileStatusMessageStabby;
use crate::SmearorWindowRotationWrapper;

fn parse_desktop_file_command_action(value: &serde_json::Value) -> DesktopFileCommandAction {
    match value.as_str() {
        Some("ExecStart") => DesktopFileCommandAction::ExecStart,
        Some("ExecReload") => DesktopFileCommandAction::ExecReload,
        Some("Terminate") => DesktopFileCommandAction::Terminate,
        _ => DesktopFileCommandAction::Exec,
    }
}

fn parse_desktop_file_status(value: &serde_json::Value) -> DesktopFileStatus {
    match value.as_str() {
        Some("Stopped") => DesktopFileStatus::Stopped,
        _ => DesktopFileStatus::Running,
    }
}

smearor_swipe_launcher_plugin_api::impl_json_convertible!(DesktopFileCommandMessageConverter, DesktopFileCommandMessage, |json: serde_json::Value| {
    let desktop_file = json.get("desktop_file").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let wrapper: Option<SmearorWindowRotationWrapper> = json.get("wrapper").and_then(|v| serde_json::from_value(v.clone()).ok());
    let action = parse_desktop_file_command_action(json.get("action").unwrap_or(&serde_json::Value::Null));
    DesktopFileCommandMessage::new(&desktop_file, wrapper, action)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(
    DesktopFileCommandMessageStabbyConverter,
    DesktopFileCommandMessageStabby,
    |json: serde_json::Value| {
        let desktop_file = json.get("desktop_file").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let wrapper: Option<SmearorWindowRotationWrapper> = json.get("wrapper").and_then(|v| serde_json::from_value(v.clone()).ok());
        let action = parse_desktop_file_command_action(json.get("action").unwrap_or(&serde_json::Value::Null));
        let msg = DesktopFileCommandMessage::new(&desktop_file, wrapper, action);
        msg.into()
    }
);

smearor_swipe_launcher_plugin_api::impl_json_convertible!(DesktopFileStatusMessageConverter, DesktopFileStatusMessage, |json: serde_json::Value| {
    let desktop_file = json.get("desktop_file").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let status = parse_desktop_file_status(json.get("status").unwrap_or(&serde_json::Value::Null));
    DesktopFileStatusMessage::new(&desktop_file, status)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(
    DesktopFileStatusMessageStabbyConverter,
    DesktopFileStatusMessageStabby,
    |json: serde_json::Value| {
        let desktop_file = json.get("desktop_file").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let status = parse_desktop_file_status(json.get("status").unwrap_or(&serde_json::Value::Null));
        let msg = DesktopFileStatusMessage::new(&desktop_file, status);
        msg.into()
    }
);

/// Register all JSON converter implementations for app-launcher messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    DesktopFileCommandMessageConverter::register_in_host(context);
    DesktopFileStatusMessageConverter::register_in_host(context);
}
