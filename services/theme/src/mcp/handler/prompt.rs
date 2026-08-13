use crate::service::ThemeService;
use smearor_model_mcp::InvokePromptError;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::PromptMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_theme_model::ThemeMcpPrompts;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for ThemeService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, sender_id: &str) {
        let prompt_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("theme: InvokePromptMessage name={} sender_id={}", prompt_name, sender_id);
        let prompt = match ThemeMcpPrompts::from_str(&prompt_name) {
            Ok(prompt) => prompt,
            Err(e) => {
                self.send_response(InvokePromptResponse::from(InvokePromptError::new(e, &correlation_id)), sender_id);
                return;
            }
        };

        let response = match prompt {
            ThemeMcpPrompts::ThemeGuide => {
                let mut content = String::from(include_str!("../../../data/prompts/theme_guide.md"));

                let state_guard = self.state.read();
                if let Ok(state) = state_guard {
                    let current = state.current_theme.as_deref().unwrap_or("none");
                    let theme_count = state.themes.len();
                    let mode = format!("{:?}", state.effective_mode);
                    content.push_str(&format!(
                        "\nCurrent snapshot:\n\
                         - Applied theme: {current}\n\
                         - Effective mode: {mode}\n\
                         - Configured themes: {theme_count}\n",
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
