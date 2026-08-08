use crate::service::VoiceAssistantService;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::PromptMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_voice_assistant_model::VoiceAssistantMcpPrompts;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, _sender_id: &str) {
        let prompt_name = message.0.name.to_string();
        debug!("Voice Assistant Service: InvokePromptMessage name={}", prompt_name);
        let broadcaster = self.get_broadcaster();
        let prompt = match VoiceAssistantMcpPrompts::from_str(&prompt_name) {
            Ok(prompt) => prompt,
            Err(_) => {
                debug!("Voice Assistant Service: ignoring InvokePromptMessage for external prompt '{prompt_name}' (handled by launcher core)");
                return;
            }
        };

        let response = match prompt {
            VoiceAssistantMcpPrompts::VoiceAssistantStatus => {
                let state = self.state.read().map(|state| format!("{:?}", *state)).unwrap_or_else(|_| "Unknown".to_string());
                let transcript = self.current_transcript.read().map(|t| t.clone()).unwrap_or_default();
                let answer = self.current_answer.read().map(|a| a.clone()).unwrap_or_default();
                let prompt_messages = vec![smearor_model_mcp::PromptMessage::new(
                    "user",
                    &format!("State: {state}\nTranscript: {transcript}\nAnswer: {answer}"),
                )];
                InvokePromptResponse::success(&message.0.correlation_id, prompt_messages)
            }
            VoiceAssistantMcpPrompts::MemoryGuide => {
                let messages = vec![PromptMessage::new("system", include_str!("../../../data/prompts/memory_guide.md"))];
                InvokePromptResponse::success(&message.0.correlation_id, messages)
            }
            VoiceAssistantMcpPrompts::ResourceDiscoveryGuide => {
                let catalog = self.resource_catalog.read().unwrap_or_else(|e| e.into_inner());
                let filter = serde_json::from_str(&message.0.arguments.to_string())
                    .unwrap_or(serde_json::Value::Null)
                    .get("filter")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                let resources: Vec<String> = catalog
                    .iter()
                    .filter(|r| {
                        filter.is_empty()
                            || r.name.to_lowercase().contains(&filter)
                            || r.uri.to_lowercase().contains(&filter)
                            || r.description.to_lowercase().contains(&filter)
                    })
                    .map(|r| format!("- {} ({}): {}", r.name, r.uri, r.description))
                    .collect();
                let content = if resources.is_empty() {
                    "No matching resources found.".to_string()
                } else {
                    format!(
                        "Available MCP resources:\n{}\n\nTo read a resource, respond with {{\"resource\": \"<uri>\"}} using the exact URI from the list above.",
                        resources.join("\n")
                    )
                };
                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&message.0.correlation_id, messages)
            }
        };
        broadcaster.broadcast_message_to_topic(response);
    }
}
