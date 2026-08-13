use crate::command::ThemeCommand;
use crate::service::ThemeService;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_theme_model::ThemeMcpTools;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for ThemeService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        let arguments_str = message.0.arguments.to_string();
        debug!("theme: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match ThemeMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &correlation_id)));
                return;
            }
        };
        match tool {
            ThemeMcpTools::GetTheme => {
                let state_guard = self.state.read();
                let (current_theme, selected_index, effective_mode, theme_count) = match state_guard {
                    Ok(s) => (s.current_theme.clone(), s.selected_theme_index, s.effective_mode, s.themes.len()),
                    Err(_) => (None, 0, smearor_theme_model::ThemeMode::Dark, 0),
                };
                let response = InvokeToolResponse::success(
                    &correlation_id,
                    &format!(
                        "Current theme: {:?}, selected index: {}, effective mode: {:?}, configured themes: {}",
                        current_theme, selected_index, effective_mode, theme_count
                    ),
                );
                self.send_response(response, sender_id);
            }
            ThemeMcpTools::SetTheme => {
                let args: serde_json::Value = serde_json::from_str(arguments_str.as_str()).unwrap_or_default();
                let theme_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if theme_name.is_empty() {
                    let response = InvokeToolResponse::error(&correlation_id, "Missing required field: name");
                    self.send_response(response, sender_id);
                } else {
                    let _ = self.command_sender.send(ThemeCommand::SelectAndApply(theme_name.to_string()));
                    let response = InvokeToolResponse::success(&correlation_id, &format!("Theme set to: {theme_name}"));
                    self.send_response(response, sender_id);
                }
            }
        }
    }
}
