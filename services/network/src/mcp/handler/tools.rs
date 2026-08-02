use crate::service::NetworkCommand;
use crate::service::NetworkService;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_network_model::NetworkMcpTools;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for NetworkService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("Network Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match NetworkMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &message.0.correlation_id)));
                return;
            }
        };
        match tool {
            NetworkMcpTools::ToggleRadio => {
                let parsed = serde_json::from_str::<serde_json::Value>(message.0.arguments.as_ref()).ok();
                let technology = parsed
                    .as_ref()
                    .and_then(|v| v.get("technology").and_then(|a| a.as_str()).map(|s| s.to_string()))
                    .unwrap_or_else(|| "wifi".to_string());
                let enabled = parsed.as_ref().and_then(|v| v.get("enabled").and_then(|a| a.as_bool())).unwrap_or(false);
                let _ = self.command_sender.send(NetworkCommand::ToggleRadio(technology, enabled));
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Radio toggle triggered");
                broadcaster.broadcast_message_to_topic(response);
            }
            NetworkMcpTools::ConnectWifi => {
                let parsed = serde_json::from_str::<serde_json::Value>(message.0.arguments.as_ref()).ok();
                let ssid = parsed
                    .as_ref()
                    .and_then(|v| v.get("ssid").and_then(|a| a.as_str()).map(|s| s.to_string()))
                    .unwrap_or_default();
                let password = parsed.as_ref().and_then(|v| v.get("password").and_then(|a| a.as_str()).map(|s| s.to_string()));
                let _ = self.command_sender.send(NetworkCommand::ConnectWifi(ssid, password));
                let response = InvokeToolResponse::success(&message.0.correlation_id, "WiFi connect triggered");
                broadcaster.broadcast_message_to_topic(response);
            }
            NetworkMcpTools::ToggleVpn => {
                let parsed = serde_json::from_str::<serde_json::Value>(message.0.arguments.as_ref()).ok();
                let profile_name = parsed
                    .as_ref()
                    .and_then(|v| v.get("profile_name").and_then(|a| a.as_str()).map(|s| s.to_string()))
                    .unwrap_or_default();
                let active = parsed.as_ref().and_then(|v| v.get("active").and_then(|a| a.as_bool())).unwrap_or(false);
                let _ = self.command_sender.send(NetworkCommand::ToggleVpn(profile_name, active));
                let response = InvokeToolResponse::success(&message.0.correlation_id, "VPN toggle triggered");
                broadcaster.broadcast_message_to_topic(response);
            }
            NetworkMcpTools::GetPublicIp => {
                let _ = self.command_sender.send(NetworkCommand::GetPublicIp);
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Public IP query triggered");
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}
