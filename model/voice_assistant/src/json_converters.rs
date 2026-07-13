use smearor_swipe_launcher_plugin_api::FfiCoreContext;

use crate::AssistantState;
use crate::AssistantStatusMessage;
use crate::AssistantStatusMessageStabby;
use crate::VoiceCommandAction;
use crate::VoiceCommandMessage;
use crate::VoiceCommandMessageStabby;

fn parse_voice_command_action(value: &serde_json::Value) -> VoiceCommandAction {
    match value.as_str() {
        Some("Deactivate") => VoiceCommandAction::Deactivate,
        Some("SubmitText") => VoiceCommandAction::SubmitText,
        _ => VoiceCommandAction::Activate,
    }
}

fn parse_assistant_state(value: &serde_json::Value) -> AssistantState {
    match value.as_str() {
        Some("Listening") => AssistantState::Listening,
        Some("ProcessingStt") => AssistantState::ProcessingStt,
        Some("ThinkingLlm") => AssistantState::ThinkingLlm,
        Some("ExecutingAction") => AssistantState::ExecutingAction,
        Some("Error") => AssistantState::Error,
        _ => AssistantState::Idle,
    }
}

smearor_swipe_launcher_plugin_api::impl_json_convertible!(VoiceCommandMessageConverter, VoiceCommandMessage, |json: serde_json::Value| {
    let action = parse_voice_command_action(json.get("action").unwrap_or(&serde_json::Value::Null));
    let text = json.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
    VoiceCommandMessage::new(action, &text)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(VoiceCommandMessageStabbyConverter, VoiceCommandMessageStabby, |json: serde_json::Value| {
    let action = parse_voice_command_action(json.get("action").unwrap_or(&serde_json::Value::Null));
    let text = json.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let msg = VoiceCommandMessage::new(action, &text);
    msg.into()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(AssistantStatusMessageConverter, AssistantStatusMessage, |json: serde_json::Value| {
    let current_state = parse_assistant_state(json.get("current_state").unwrap_or(&serde_json::Value::Null));
    let partial_transcript = json.get("partial_transcript").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let final_answer = json.get("final_answer").and_then(|v| v.as_str()).map(|s| s.to_string());
    let active_tool = json.get("active_tool").and_then(|v| v.as_str()).map(|s| s.to_string());
    let error_message = json.get("error_message").and_then(|v| v.as_str()).map(|s| s.to_string());
    AssistantStatusMessage {
        current_state,
        partial_transcript,
        final_answer,
        active_tool,
        error_message,
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(AssistantStatusMessageStabbyConverter, AssistantStatusMessageStabby, |json: serde_json::Value| {
    let current_state = parse_assistant_state(json.get("current_state").unwrap_or(&serde_json::Value::Null));
    let partial_transcript = json.get("partial_transcript").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let final_answer = json.get("final_answer").and_then(|v| v.as_str()).map(|s| s.to_string());
    let active_tool = json.get("active_tool").and_then(|v| v.as_str()).map(|s| s.to_string());
    let error_message = json.get("error_message").and_then(|v| v.as_str()).map(|s| s.to_string());
    let msg = AssistantStatusMessage {
        current_state,
        partial_transcript,
        final_answer,
        active_tool,
        error_message,
    };
    msg.into()
});

/// Register all JSON converter implementations for voice assistant messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    VoiceCommandMessageConverter::register_in_host(context);
    AssistantStatusMessageConverter::register_in_host(context);
}
