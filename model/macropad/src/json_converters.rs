use crate::MacroPadCommand;
use crate::MacroPadCommandMessage;
use crate::MacroPadCommandType;
use crate::MacroPadConnectionStatus;
use crate::MacroPadInputMessage;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(MacroPadInputMessageConverter, MacroPadInputMessage, |json: serde_json::Value| {
    let device_id = json.get("device_id").and_then(|v| v.as_str()).unwrap_or("");
    let instance_id = json.get("instance_id").and_then(|v| v.as_str()).unwrap_or("");
    let button_index = json.get("button_index").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let pressed = json.get("pressed").and_then(|v| v.as_bool()).unwrap_or(false);
    MacroPadInputMessage::new(device_id, instance_id, button_index, pressed)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(MacroPadConnectionStatusConverter, MacroPadConnectionStatus, |json: serde_json::Value| {
    let device_id = json.get("device_id").and_then(|v| v.as_str()).unwrap_or("");
    let instance_id = json.get("instance_id").and_then(|v| v.as_str()).unwrap_or("");
    let device_type = json.get("device_type").and_then(|v| v.as_str()).unwrap_or("");
    let driver = json.get("driver").and_then(|v| v.as_str()).unwrap_or("");
    let key_count = json.get("key_count").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let key_columns = json.get("key_columns").and_then(|v| v.as_u64()).unwrap_or(key_count as u64) as u8;
    let key_width = json.get("key_width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let key_height = json.get("key_height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let connected = json.get("connected").and_then(|v| v.as_bool()).unwrap_or(false);
    MacroPadConnectionStatus::new(device_id, instance_id, device_type, driver, key_count, key_columns, key_width, key_height, connected)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(MacroPadCommandMessageConverter, MacroPadCommandMessage, |json: serde_json::Value| {
    let device_id = json.get("device_id").and_then(|v| v.as_str()).unwrap_or("");
    let command_json = json.get("command").unwrap_or(&serde_json::Value::Null);
    let command_type_str = command_json.get("type").and_then(|v| v.as_str()).unwrap_or("ClearAllButtons");
    let command_type = match command_type_str {
        "SetBrightness" => MacroPadCommandType::SetBrightness,
        "ClearAllButtons" => MacroPadCommandType::ClearAllButtons,
        "ClearButton" => MacroPadCommandType::ClearButton,
        "SetButtonImage" => MacroPadCommandType::SetButtonImage,
        "Reset" => MacroPadCommandType::Reset,
        _ => MacroPadCommandType::ClearAllButtons,
    };
    let percent = command_json.get("percent").and_then(|v| v.as_u64()).unwrap_or(50) as u8;
    let button_index = command_json.get("button_index").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let width = command_json.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let height = command_json.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let pixels: Vec<u8> = command_json
        .get("pixels")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect())
        .unwrap_or_default();
    let command = MacroPadCommand {
        command_type,
        percent,
        button_index,
        width,
        height,
        pixels: stabby::vec::Vec::from(pixels.as_slice()),
    };
    MacroPadCommandMessage::new(device_id, command)
});

/// Register all JSON converter implementations for MacroPad messages.
///
/// Call this once during startup.
pub fn register_json_converters(context: Option<smearor_swipe_launcher_plugin_api::FfiCoreContext>) {
    MacroPadInputMessageConverter::register_in_host(context);
    MacroPadConnectionStatusConverter::register_in_host(context);
    MacroPadCommandMessageConverter::register_in_host(context);
}
