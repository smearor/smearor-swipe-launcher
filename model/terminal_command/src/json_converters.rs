use smearor_swipe_launcher_plugin_api::FfiCoreContext;

use crate::TerminalCommandAction;
use crate::TerminalCommandMessage;
use crate::TerminalCommandMessageStabby;
use crate::TerminalCommandStatus;
use crate::TerminalCommandStatusMessage;
use crate::TerminalCommandStatusMessageStabby;

fn parse_terminal_command_action(value: &serde_json::Value) -> TerminalCommandAction {
    match value.as_str() {
        Some("Terminate") => TerminalCommandAction::Terminate,
        Some("Restart") => TerminalCommandAction::Restart,
        _ => TerminalCommandAction::Launch,
    }
}

fn parse_terminal_command_status(value: &serde_json::Value) -> TerminalCommandStatus {
    match value.as_str() {
        Some("Stopped") => TerminalCommandStatus::Stopped,
        Some("Failed") => TerminalCommandStatus::Failed,
        _ => TerminalCommandStatus::Running,
    }
}

smearor_swipe_launcher_plugin_api::impl_json_convertible!(TerminalCommandMessageConverter, TerminalCommandMessage, |json: serde_json::Value| {
    let command_id = json.get("command_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let action = parse_terminal_command_action(json.get("action").unwrap_or(&serde_json::Value::Null));
    let forked = json.get("forked").and_then(|v| v.as_bool()).unwrap_or(false);
    let terminate_on_exit = json.get("terminate_on_exit").and_then(|v| v.as_bool()).unwrap_or(true);
    TerminalCommandMessage::new(&command_id, action, forked, terminate_on_exit)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(TerminalCommandMessageStabbyConverter, TerminalCommandMessageStabby, |json: serde_json::Value| {
    let command_id = json.get("command_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let action = parse_terminal_command_action(json.get("action").unwrap_or(&serde_json::Value::Null));
    let forked = json.get("forked").and_then(|v| v.as_bool()).unwrap_or(false);
    let terminate_on_exit = json.get("terminate_on_exit").and_then(|v| v.as_bool()).unwrap_or(true);
    let msg = TerminalCommandMessage::new(&command_id, action, forked, terminate_on_exit);
    msg.into()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(TerminalCommandStatusMessageConverter, TerminalCommandStatusMessage, |json: serde_json::Value| {
    let command_id = json.get("command_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let status = parse_terminal_command_status(json.get("status").unwrap_or(&serde_json::Value::Null));
    let pid = json.get("pid").and_then(|v| v.as_u64()).map(|p| p as u32);
    TerminalCommandStatusMessage::new(&command_id, status, pid)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(
    TerminalCommandStatusMessageStabbyConverter,
    TerminalCommandStatusMessageStabby,
    |json: serde_json::Value| {
        let command_id = json.get("command_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let status = parse_terminal_command_status(json.get("status").unwrap_or(&serde_json::Value::Null));
        let pid = json.get("pid").and_then(|v| v.as_u64()).map(|p| p as u32);
        let msg = TerminalCommandStatusMessage::new(&command_id, status, pid);
        msg.into()
    }
);

/// Register all JSON converter implementations for terminal-command messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    TerminalCommandMessageConverter::register_in_host(context);
    TerminalCommandStatusMessageConverter::register_in_host(context);
}
