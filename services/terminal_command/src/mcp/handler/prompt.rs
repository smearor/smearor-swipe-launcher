use crate::service::TerminalCommandService;
use smearor_model_mcp::InvokePromptError;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::PromptMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_terminal_command_model::TerminalCommandMcpPrompts;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for TerminalCommandService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, _sender_id: &str) {
        let prompt_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("TerminalCommand Service: InvokePromptMessage name={}", prompt_name);
        let broadcaster = self.get_broadcaster();
        let prompt = match TerminalCommandMcpPrompts::from_str(&prompt_name) {
            Ok(prompt) => prompt,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokePromptResponse::from(InvokePromptError::new(e, &correlation_id)));
                return;
            }
        };

        let response = match prompt {
            TerminalCommandMcpPrompts::TerminalCommandGuide => {
                let configured = self.configured_commands_snapshot();
                let running = self.running_commands_snapshot();

                let configured_list = if configured.is_empty() {
                    "No terminal commands configured.".to_string()
                } else {
                    let items: Vec<String> = configured
                        .iter()
                        .map(|(id, cmd, args, restart)| {
                            let arg_str = if args.is_empty() { String::new() } else { format!(" {}", args.join(" ")) };
                            format!("- {id}: {cmd}{arg_str} (restart_on_exit={restart})")
                        })
                        .collect();
                    format!("Configured commands:\n{}", items.join("\n"))
                };

                let running_list = if running.is_empty() {
                    "No commands currently running.".to_string()
                } else {
                    let items: Vec<String> = running
                        .iter()
                        .map(|(id, pids, terminate_on_exit)| format!("- {id}: PIDs {:?}, terminate_on_exit={terminate_on_exit}", pids))
                        .collect();
                    format!("Running commands:\n{}", items.join("\n"))
                };

                let content = format!(
                    "{configured_list}\n\n{running_list}\n\n\
                     Tools:\n\
                     - terminal_command_launch: Launch a configured command by command_id\n\
                     - terminal_command_terminate: Terminate a running command by command_id\n\
                     - terminal_command_restart: Restart a command (terminate then launch)\n\n\
                     Resources:\n\
                     - terminal_command://running: List running tracked commands\n\
                     - terminal_command://configured: List all configured commands"
                );

                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
            TerminalCommandMcpPrompts::TerminalLifecycleGuide => {
                let configured = self.configured_commands_snapshot();
                let running = self.running_commands_snapshot();

                let configured_ids: Vec<String> = configured.iter().map(|(id, _, _, _)| id.clone()).collect();
                let running_ids: Vec<String> = running.iter().map(|(id, _, _)| id.clone()).collect();

                let content = format!(
                    "Terminal command lifecycle guide:\n\
                     \n\
                     1. BEFORE LAUNCH: Always read terminal_command://configured to verify the command_id exists.\n\
                     2. LAUNCH: Use terminal_command_launch with the correct command_id. Check forked and terminate_on_exit parameters.\n\
                     3. MONITOR: Read terminal_command://running to check if the process is still active.\n\
                     4. TERMINATE: Use terminal_command_terminate to stop a running command by command_id.\n\
                     5. RESTART: Use terminal_command_restart to terminate and relaunch a command.\n\
                     \n\
                     Configured command IDs: {}\n\
                     Currently running: {}\n\
                     \n\
                     Important: Never launch a command without verifying it exists in the configuration first.\
                     If a command has restart_on_exit=true, it will automatically restart after termination.",
                    if configured_ids.is_empty() {
                        "none".to_string()
                    } else {
                        configured_ids.join(", ")
                    },
                    if running_ids.is_empty() { "none".to_string() } else { running_ids.join(", ") },
                );

                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
        };
        broadcaster.broadcast_message_to_topic(response);
    }
}
