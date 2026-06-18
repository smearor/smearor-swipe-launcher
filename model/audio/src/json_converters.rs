use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use stabby::option::Option as StabbyOption;
use stabby::vec::Vec as StabbyVec;

use crate::AudioCommandAction;
use crate::AudioCommandMessage;
use crate::AudioDevice;
use crate::AudioStatusMessage;

fn parse_audio_command_action(value: &serde_json::Value) -> AudioCommandAction {
    match value.as_str() {
        Some("VolumeDown") => AudioCommandAction::VolumeDown,
        Some("SetVolume") => AudioCommandAction::SetVolume,
        Some("ToggleMute") => AudioCommandAction::ToggleMute,
        Some("Mute") => AudioCommandAction::Mute,
        Some("Unmute") => AudioCommandAction::Unmute,
        Some("NextDevice") => AudioCommandAction::NextDevice,
        Some("PreviousDevice") => AudioCommandAction::PreviousDevice,
        _ => AudioCommandAction::VolumeUp,
    }
}

fn parse_audio_device(value: &serde_json::Value) -> AudioDevice {
    AudioDevice {
        id: value.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        name: value.get("name").and_then(|v| v.as_str()).unwrap_or("").into(),
        is_default: value.get("is_default").and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

fn parse_audio_devices(value: &serde_json::Value) -> StabbyVec<AudioDevice> {
    let mut devices = StabbyVec::new();
    if let Some(arr) = value.as_array() {
        for item in arr {
            devices.push(parse_audio_device(item));
        }
    }
    devices
}

smearor_swipe_launcher_plugin_api::impl_json_convertible!(AudioCommandMessageConverter, AudioCommandMessage, |json: serde_json::Value| {
    let action = parse_audio_command_action(json.get("action").unwrap_or(&serde_json::Value::Null));
    let volume = json.get("volume").and_then(|v| v.as_f64()).map(|v| v as f32);
    let device_id = json.get("device_id").and_then(|v| v.as_u64()).map(|v| v as u32);
    AudioCommandMessage::new(
        action,
        volume.map(StabbyOption::Some).unwrap_or(StabbyOption::None()),
        device_id.map(StabbyOption::Some).unwrap_or(StabbyOption::None()),
    )
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(AudioStatusMessageConverter, AudioStatusMessage, |json: serde_json::Value| {
    let volume = json.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    let is_muted = json.get("is_muted").and_then(|v| v.as_bool()).unwrap_or(false);
    let output_devices = parse_audio_devices(json.get("output_devices").unwrap_or(&serde_json::Value::Null));
    let input_devices = parse_audio_devices(json.get("input_devices").unwrap_or(&serde_json::Value::Null));
    let active_device = json
        .get("active_device")
        .and_then(|v| if v.is_null() { None } else { Some(parse_audio_device(v)) })
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None());
    AudioStatusMessage::new(volume, is_muted, output_devices, input_devices, active_device)
});

/// Register all JSON converter implementations for audio messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    AudioCommandMessageConverter::register_in_host(context);
    AudioStatusMessageConverter::register_in_host(context);
}
