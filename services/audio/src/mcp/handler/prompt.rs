use crate::service::AudioService;
use smearor_audio_model::AudioMcpPrompts;
use smearor_model_mcp::InvokePromptError;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::PromptMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for AudioService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, sender_id: &str) {
        let prompt_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("audio: InvokePromptMessage name={} sender_id={}", prompt_name, sender_id);
        let prompt = match AudioMcpPrompts::from_str(&prompt_name) {
            Ok(prompt) => prompt,
            Err(e) => {
                self.send_response(InvokePromptResponse::from(InvokePromptError::new(e, &correlation_id)), sender_id);
                return;
            }
        };

        let response = match prompt {
            AudioMcpPrompts::AudioControlGuide => {
                let status = self.last_status.lock().map(|s| s.clone()).unwrap_or(None);
                let mut content = String::from(include_str!("../../../data/prompts/audio_control_guide.md"));

                if let Some(status) = status {
                    let volume_percent = (status.volume * 100.0).round() as i32;
                    let mute_str = if status.is_muted { "muted" } else { "not muted" };
                    let active_device = status
                        .active_device
                        .as_ref()
                        .map(|d| d.name.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let device_count = status.output_devices.len();
                    content.push_str(&format!(
                        "\nCurrent snapshot:\n\
                         - Volume: {}% ({})\n\
                         - Active device: {}\n\
                         - Available output devices: {}\n",
                        volume_percent, mute_str, active_device, device_count
                    ));
                } else {
                    content.push_str("\nCurrent status: unavailable (no status received yet)\n");
                }

                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
        };

        self.send_response(response, sender_id);
    }
}
