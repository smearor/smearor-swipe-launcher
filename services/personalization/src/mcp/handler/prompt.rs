use crate::service::PersonalizationService;
use smearor_model_mcp::InvokePromptError;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::PromptMessage;
use smearor_personalization_model::PersonalizationMcpPrompts;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for PersonalizationService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, sender_id: &str) {
        let prompt_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("personalization: InvokePromptMessage name={} sender_id={}", prompt_name, sender_id);
        let prompt = match PersonalizationMcpPrompts::from_str(&prompt_name) {
            Ok(prompt) => prompt,
            Err(e) => {
                self.send_response(InvokePromptResponse::from(InvokePromptError::new(e, &correlation_id)), sender_id);
                return;
            }
        };

        let response = match prompt {
            PersonalizationMcpPrompts::PersonalizationGuide => {
                let mut content = String::from(include_str!("../../../data/prompts/personalization_guide.md"));

                if let Ok(state) = self.latest_state.read() {
                    let status = &state.status;
                    let coords = status
                        .coordinates
                        .as_ref()
                        .map(|c| {
                            let name = c.location_name.as_ref().map(|n| n.to_string()).unwrap_or_else(|| "unknown".to_string());
                            format!("({}, {}) - {}", c.latitude, c.longitude, name)
                        })
                        .unwrap_or_else(|| "unknown".to_string());
                    let timezone = status.timezone.as_ref().map(|t| t.to_string()).unwrap_or_else(|| "unknown".to_string());
                    let locale = status.locale.as_ref().map(|l| l.to_string()).unwrap_or_else(|| "unknown".to_string());
                    content.push_str(&format!(
                        "\nCurrent snapshot:\n\
                         - Location: {coords}\n\
                         - Timezone: {timezone}\n\
                         - Locale: {locale}\n\
                         - Temperature unit: {:?}\n\
                         - Measurement system: {:?}\n",
                        status.temperature_unit, status.measurement_system,
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
