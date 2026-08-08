use crate::service::WallpaperService;
use smearor_model_mcp::InvokePromptError;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::PromptMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_wallpaper_model::WallpaperMcpPrompts;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for WallpaperService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, sender_id: &str) {
        let prompt_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("wallpaper: InvokePromptMessage name={} sender_id={}", prompt_name, sender_id);
        let prompt = match WallpaperMcpPrompts::from_str(&prompt_name) {
            Ok(prompt) => prompt,
            Err(e) => {
                self.send_response(InvokePromptResponse::from(InvokePromptError::new(e, &correlation_id)), sender_id);
                return;
            }
        };

        let response = match prompt {
            WallpaperMcpPrompts::WallpaperGuide => {
                let mut content = String::from(include_str!("../../../data/prompts/wallpaper_guide.md"));

                let theme_count = self.config.read().map(|c| c.themes.len()).unwrap_or(0);
                if let Ok(state) = self.state.read() {
                    let running = state.current_theme.as_deref().unwrap_or("none");
                    let process_count = state.current_processes.len();
                    content.push_str(&format!(
                        "\nCurrent snapshot:\n\
                         - Running theme: {running}\n\
                         - Active processes: {process_count}\n\
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
