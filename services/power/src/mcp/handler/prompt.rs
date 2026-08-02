use crate::service::PowerService;
use smearor_model_mcp::InvokePromptError;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::PromptMessage;
use smearor_power_model::PowerMcpPrompts;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::trace;

impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for PowerService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, _sender_id: &str) {
        let prompt_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        trace!("Power Service: InvokePromptMessage name={}", prompt_name);
        let broadcaster = self.get_broadcaster();
        let prompt = match PowerMcpPrompts::from_str(&prompt_name) {
            Ok(prompt) => prompt,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokePromptResponse::from(InvokePromptError::new(e, &correlation_id)));
                return;
            }
        };

        let response = match prompt {
            PowerMcpPrompts::PowerActionGuide => {
                let state = self.state_snapshot();
                let caps = &state.capabilities;
                let capabilities = format!(
                    "System capabilities: shutdown={}, reboot={}, suspend={}, hibernate={}, reboot_to_uefi={}, lock={}, logout={}",
                    caps.can_shutdown, caps.can_reboot, caps.can_suspend, caps.can_hibernate, caps.can_reboot_to_firmware, caps.can_lock, caps.can_logout
                );

                let inhibitors_info = if state.inhibitors.is_empty() {
                    "No inhibitors blocking power actions.".to_string()
                } else {
                    let items: Vec<String> = state
                        .inhibitors
                        .iter()
                        .map(|inh| format!("- {} ({}): {}", inh.process_name.to_string(), inh.what.to_string(), inh.reason.to_string()))
                        .collect();
                    format!("Active inhibitors:\n{}", items.join("\n"))
                };

                let scheduled_info = match state.scheduled_action.as_ref() {
                    Some(sched) => format!("Scheduled action: {:?} in {} seconds", sched.action, sched.remaining_seconds),
                    None => "No scheduled power actions.".to_string(),
                };

                let content = format!(
                    "{capabilities}\n{inhibitors_info}\n{scheduled_info}\n\n\
                     Tools:\n\
                     - system_power_action: Execute a power action immediately (shutdown, reboot, suspend, hibernate, lock, logout)\n\
                     - system_schedule_power_action: Schedule a shutdown or reboot in N minutes\n\
                     - system_cancel_power_action: Cancel a running countdown or scheduled action\n\
                     - system_reboot_to_uefi: Reboot directly into BIOS/UEFI\n\n\
                     Resources:\n\
                     - power://capabilities: System power capabilities\n\
                     - power://inhibitors: Active inhibitor locks\n\
                     - power://scheduled_actions: Currently scheduled power action\n\n\
                     Safety: Always check inhibitors before executing power actions. Warn the user about unsaved work."
                );

                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
            PowerMcpPrompts::PowerSafetyGuide => {
                let state = self.state_snapshot();
                let inhibitors = &state.inhibitors;

                let inhibitor_warning = if inhibitors.is_empty() {
                    "No inhibitors are currently blocking power actions.".to_string()
                } else {
                    let items: Vec<String> = inhibitors
                        .iter()
                        .map(|inh| format!("- {} ({}): {}", inh.process_name.to_string(), inh.what.to_string(), inh.reason.to_string()))
                        .collect();
                    format!(
                        "WARNING: Active inhibitors are blocking power actions:\n{}\nThe user should be informed about these inhibitors.",
                        items.join("\n")
                    )
                };

                let content = format!(
                    "Power safety guide:\n\n\
                     CRITICAL RULES:\n\
                     1. ALWAYS ask for user confirmation before executing any of these actions:\n\
                        - shutdown: Power off the system completely\n\
                        - reboot: Restart the system\n\
                        - reboot_to_uefi: Restart into BIOS/UEFI firmware\n\
                        - hibernate: Save state to disk and power off\n\
                     2. For suspend and lock, confirmation is recommended but not strictly required.\n\
                     3. Before executing, warn the user about unsaved work in other applications.\n\
                     4. Check power://inhibitors for active inhibitor locks. If inhibitors exist, inform the user.\n\
                     5. For scheduled actions (system_schedule_power_action), state the delay clearly and offer cancellation via system_cancel_power_action.\n\
                     6. After executing a destructive action, there is NO undo. Be certain before proceeding.\n\n\
                     Confirmation format:\n\
                     \"I will <action> the system in <delay> seconds. Unsaved work will be lost. Proceed?\"\n\n\
                     {}",
                    inhibitor_warning
                );

                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
        };
        broadcaster.broadcast_message_to_topic(response);
    }
}
