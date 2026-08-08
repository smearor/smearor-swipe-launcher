use crate::service::NetworkService;
use smearor_model_mcp::InvokePromptError;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::PromptMessage;
use smearor_network_model::NetworkMcpPrompts;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for NetworkService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, sender_id: &str) {
        let prompt_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("network: InvokePromptMessage name={} sender_id={}", prompt_name, sender_id);
        let prompt = match NetworkMcpPrompts::from_str(&prompt_name) {
            Ok(prompt) => prompt,
            Err(e) => {
                self.send_response(InvokePromptResponse::from(InvokePromptError::new(e, &correlation_id)), sender_id);
                return;
            }
        };

        let response = match prompt {
            NetworkMcpPrompts::NetworkGuide => {
                let mut content = String::from(include_str!("../../../data/prompts/network_guide.md"));

                if let Ok(state) = self.shared_state.lock() {
                    let status = &state.status;
                    let iface = &status.primary_interface;
                    let ssid = iface.ssid.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "N/A".to_string());
                    let signal = iface.signal.as_ref().map(|s| format!("{}%", s)).unwrap_or_else(|| "N/A".to_string());
                    let ip = iface.ipv4_address.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "N/A".to_string());
                    let radio = if status.wifi_enabled { "enabled" } else { "disabled" };
                    let vpn_count = state.vpn_profiles.profiles.len();
                    let active_vpns = state.vpn_profiles.profiles.iter().filter(|p| p.is_active).count();
                    content.push_str(&format!(
                        "\nCurrent snapshot:\n\
                         - WiFi: {radio}\n\
                         - SSID: {ssid}\n\
                         - Signal: {signal}\n\
                         - IP: {ip}\n\
                         - VPN profiles: {vpn_count} ({active_vpns} active)\n",
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
