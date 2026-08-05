use crate::service::DoaService;
use smearor_doa_model::DoaCommandAction;
use smearor_doa_model::DoaCommandMessage;
use smearor_doa_model::DoaDirectionResponse;
use smearor_doa_model::DoaMcpTools;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for DoaService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("DoA Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match DoaMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &correlation_id)));
                return;
            }
        };
        match tool {
            DoaMcpTools::GetDirection => {
                let state = self.state_snapshot();
                let response_payload = DoaDirectionResponse::from(state);
                let json = serde_json::to_string(&response_payload).unwrap_or_else(|e| {
                    debug!("DoA Service: failed to serialize DoaDirectionResponse: {e}");
                    format!("{{\"error\":\"Serialization failed: {e}\"}}")
                });
                let response = InvokeToolResponse::success(&correlation_id, &json);
                broadcaster.broadcast_message_to_topic(response);
            }
            DoaMcpTools::SetPollInterval => {
                let args_result = serde_json::from_str::<serde_json::Value>(&message.0.arguments.to_string());
                match args_result {
                    Ok(args) => {
                        let interval = args.get("interval_ms").and_then(|v| v.as_u64()).unwrap_or(150).max(50);
                        let cmd = DoaCommandMessage {
                            action: DoaCommandAction::SetPollInterval,
                            value: interval,
                        };
                        let _ = self.command_sender.send(cmd);
                        let response = InvokeToolResponse::success(&correlation_id, &format!("Poll interval set to {}ms", interval));
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    Err(parse_error) => {
                        debug!("DoA Service: doa_set_poll_interval argument parse error: {parse_error}");
                        let response = InvokeToolResponse::error(&correlation_id, &format!("Invalid arguments: {parse_error}"));
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            DoaMcpTools::Reconnect => {
                let cmd = DoaCommandMessage {
                    action: DoaCommandAction::Reconnect,
                    value: 0,
                };
                let _ = self.command_sender.send(cmd);
                let response = InvokeToolResponse::success(&correlation_id, "Reconnection initiated");
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}
