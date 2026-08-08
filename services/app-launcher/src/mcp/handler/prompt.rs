use crate::service::AppLauncherService;
use smearor_app_launcher_model::AppLauncherMcpPrompts;
use smearor_model_mcp::InvokePromptError;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::PromptMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for AppLauncherService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, _sender_id: &str) {
        let prompt_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("AppLauncher Service: InvokePromptMessage name={}", prompt_name);
        let broadcaster = self.get_broadcaster();
        let prompt = match AppLauncherMcpPrompts::from_str(&prompt_name) {
            Ok(prompt) => prompt,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokePromptResponse::from(InvokePromptError::new(e, &correlation_id)));
                return;
            }
        };

        let response = match prompt {
            AppLauncherMcpPrompts::AppLaunchGuide => {
                let running = self.running_apps_snapshot();
                let running_list = if running.is_empty() {
                    "No applications currently running.".to_string()
                } else {
                    let items: Vec<String> = running
                        .iter()
                        .map(|(desktop_file, pids, terminate_on_exit)| format!("- {desktop_file}: PIDs {:?}, terminate_on_exit={terminate_on_exit}", pids))
                        .collect();
                    format!("Running applications:\n{}", items.join("\n"))
                };

                let content = format!("{running_list}\n\n{}", include_str!("../../../data/prompts/app_launch_guide.md"));

                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
        };
        broadcaster.broadcast_message_to_topic(response);
    }
}
