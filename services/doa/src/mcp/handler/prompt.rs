use crate::service::DoaService;
use smearor_doa_model::DoaMcpPrompts;
use smearor_model_mcp::InvokePromptError;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::PromptMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for DoaService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, sender_id: &str) {
        let prompt_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("doa: InvokePromptMessage name={} sender_id={}", prompt_name, sender_id);
        let prompt = match DoaMcpPrompts::from_str(&prompt_name) {
            Ok(prompt) => prompt,
            Err(e) => {
                self.send_response(InvokePromptResponse::from(InvokePromptError::new(e, &correlation_id)), sender_id);
                return;
            }
        };

        let response = match prompt {
            DoaMcpPrompts::DoaGuide => {
                let mut content = String::from(include_str!("../../../data/prompts/doa_guide.md"));

                if let Ok(state) = self.shared_state.lock() {
                    let connected = if state.connected { "connected" } else { "disconnected" };
                    let paused = if state.paused { "paused" } else { "active" };
                    let speech = if state.speech_detected { "detected" } else { "none" };
                    content.push_str(&format!(
                        "\nCurrent snapshot:\n\
                         - Device: {connected} ({paused})\n\
                         - Angle: {}° (calibrated: {}°)\n\
                         - Rotation offset: {}°\n\
                         - Speech: {speech}\n\
                         - Last updated: {}\n",
                        state.angle, state.calibrated_angle, state.rotation_offset, state.last_updated,
                    ));
                } else {
                    content.push_str("\nCurrent status: unavailable\n");
                }

                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
        };

        self.send_response(response, sender_id);
    }
}
