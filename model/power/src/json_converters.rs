use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use stabby::option::Option as StabbyOption;
use stabby::vec::Vec as StabbyVec;

use crate::InhibitorInfo;
use crate::PowerAction;
use crate::PowerCapabilities;
use crate::PowerCommandAction;
use crate::PowerCommandMessage;
use crate::PowerStatusMessage;
use crate::ScheduledActionInfo;

fn parse_power_action(value: &serde_json::Value) -> PowerAction {
    match value.as_str() {
        Some("Shutdown") => PowerAction::Shutdown,
        Some("Reboot") => PowerAction::Reboot,
        Some("Suspend") => PowerAction::Suspend,
        Some("Hibernate") => PowerAction::Hibernate,
        Some("Lock") => PowerAction::Lock,
        Some("Logout") => PowerAction::Logout,
        Some("RebootToFirmware") => PowerAction::RebootToFirmware,
        _ => PowerAction::Cancel,
    }
}

fn parse_power_command_action(value: &serde_json::Value) -> PowerCommandAction {
    match value.as_str() {
        Some("Schedule") => PowerCommandAction::Schedule,
        Some("Cancel") => PowerCommandAction::Cancel,
        Some("Refresh") => PowerCommandAction::Refresh,
        _ => PowerCommandAction::Execute,
    }
}

fn parse_capabilities(value: &serde_json::Value) -> PowerCapabilities {
    PowerCapabilities {
        can_shutdown: value.get("can_shutdown").and_then(|v| v.as_bool()).unwrap_or(false),
        can_reboot: value.get("can_reboot").and_then(|v| v.as_bool()).unwrap_or(false),
        can_suspend: value.get("can_suspend").and_then(|v| v.as_bool()).unwrap_or(false),
        can_hibernate: value.get("can_hibernate").and_then(|v| v.as_bool()).unwrap_or(false),
        can_reboot_to_firmware: value.get("can_reboot_to_firmware").and_then(|v| v.as_bool()).unwrap_or(false),
        can_lock: value.get("can_lock").and_then(|v| v.as_bool()).unwrap_or(false),
        can_logout: value.get("can_logout").and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

fn parse_inhibitor(value: &serde_json::Value) -> InhibitorInfo {
    InhibitorInfo {
        process_name: stabby::string::String::from(value.get("process_name").and_then(|v| v.as_str()).unwrap_or("")),
        reason: stabby::string::String::from(value.get("reason").and_then(|v| v.as_str()).unwrap_or("")),
        what: stabby::string::String::from(value.get("what").and_then(|v| v.as_str()).unwrap_or("")),
        who: stabby::string::String::from(value.get("who").and_then(|v| v.as_str()).unwrap_or("")),
    }
}

fn parse_inhibitors(value: &serde_json::Value) -> StabbyVec<InhibitorInfo> {
    let mut inhibitors = StabbyVec::new();
    if let Some(arr) = value.as_array() {
        for item in arr {
            inhibitors.push(parse_inhibitor(item));
        }
    }
    inhibitors
}

fn parse_scheduled_action(value: &serde_json::Value) -> StabbyOption<ScheduledActionInfo> {
    if value.is_null() {
        return StabbyOption::None();
    }
    StabbyOption::Some(ScheduledActionInfo {
        action: parse_power_action(value.get("action").unwrap_or(&serde_json::Value::Null)),
        remaining_seconds: value.get("remaining_seconds").and_then(|v| v.as_u64()).unwrap_or(0),
        total_delay_seconds: value.get("total_delay_seconds").and_then(|v| v.as_u64()).unwrap_or(0),
    })
}

smearor_swipe_launcher_plugin_api::impl_json_convertible!(PowerCommandMessageConverter, PowerCommandMessage, |json: serde_json::Value| {
    let action = parse_power_command_action(json.get("action").unwrap_or(&serde_json::Value::Null));
    let power_action = parse_power_action(json.get("power_action").unwrap_or(&serde_json::Value::Null));
    let delay_minutes = json.get("delay_minutes").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    PowerCommandMessage::new(action, power_action, delay_minutes)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(PowerStatusMessageConverter, PowerStatusMessage, |json: serde_json::Value| {
    let capabilities = parse_capabilities(json.get("capabilities").unwrap_or(&serde_json::Value::Null));
    let inhibitors = parse_inhibitors(json.get("inhibitors").unwrap_or(&serde_json::Value::Null));
    let scheduled_action = parse_scheduled_action(json.get("scheduled_action").unwrap_or(&serde_json::Value::Null));
    let countdown_active = json.get("countdown_active").and_then(|v| v.as_bool()).unwrap_or(false);
    let countdown_remaining_seconds = json.get("countdown_remaining_seconds").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let countdown_action = parse_power_action(json.get("countdown_action").unwrap_or(&serde_json::Value::Null));
    let last_updated = stabby::string::String::from(json.get("last_updated").and_then(|v| v.as_str()).unwrap_or(""));
    PowerStatusMessage::new(
        capabilities,
        inhibitors,
        scheduled_action,
        countdown_active,
        countdown_remaining_seconds,
        countdown_action,
        last_updated,
    )
});

/// Register all JSON converter implementations for power messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    PowerCommandMessageConverter::register_in_host(context);
    PowerStatusMessageConverter::register_in_host(context);
}
