use crate::service::SysinfoService;
use smearor_model_mcp::InvokePromptError;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::PromptMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_sysinfo_model::SysinfoMcpPrompts;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for SysinfoService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, sender_id: &str) {
        let prompt_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("sysinfo: InvokePromptMessage name={} sender_id={}", prompt_name, sender_id);
        let prompt = match SysinfoMcpPrompts::from_str(&prompt_name) {
            Ok(prompt) => prompt,
            Err(e) => {
                self.send_response(InvokePromptResponse::from(InvokePromptError::new(e, &correlation_id)), sender_id);
                return;
            }
        };

        let response = match prompt {
            SysinfoMcpPrompts::SystemHealthCheck => {
                let state = match self.latest_state.read() {
                    Ok(state) => state.clone(),
                    Err(_) => {
                        let response = InvokePromptResponse::error(&correlation_id, "Failed to read sysinfo state");
                        self.send_response(response, sender_id);
                        return;
                    }
                };

                let cpu_usage = state.cpu.cpu_usage;
                let cpu_temp = state.cpu.cpu_temperature.as_ref().copied().unwrap_or(0.0);
                let mem_used = state.memory.memory_used;
                let mem_total = state.memory.memory_total;
                let mem_percent = if mem_total > 0 { (mem_used as f64 / mem_total as f64) * 100.0 } else { 0.0 };
                let battery_level = state.battery.level;
                let battery_state = format!("{:?}", state.battery.status);
                let uptime_seconds = state.uptime.uptime_seconds;

                let temp_warning = if cpu_temp > 80.0 {
                    format!("WARNING: CPU temperature is high ({:.1}°C). Check cooling.", cpu_temp)
                } else if cpu_temp > 0.0 {
                    format!("CPU temperature is normal ({:.1}°C).", cpu_temp)
                } else {
                    "CPU temperature not available.".to_string()
                };

                let battery_info = if battery_level >= 0.0 {
                    format!("Battery: {:.0}% ({})", battery_level, battery_state)
                } else {
                    "Battery: not available.".to_string()
                };

                let content = format!(
                    "{}\n\n\
                     Current snapshot:\n\
                     - CPU usage: {:.1}%\n\
                     - CPU temperature: {:.1}°C\n\
                     - Memory: {} / {} ({:.1}%)\n\
                     - {}\n\
                     - {}\n\
                     - Uptime: {} seconds\n",
                    include_str!("../../../data/prompts/system_health_check.md"),
                    cpu_usage,
                    cpu_temp,
                    mem_used,
                    mem_total,
                    mem_percent,
                    temp_warning,
                    battery_info,
                    uptime_seconds
                );

                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
        };
        self.send_response(response, sender_id);
    }
}
