use crate::service::StreamDeckService;
use smearor_model_macropad::MacroPadMcpTools;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for StreamDeckService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("StreamDeck Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match MacroPadMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &message.0.correlation_id)));
                return;
            }
        };
        match tool {
            MacroPadMcpTools::StreamDeckSetBrightness => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let brightness = args.get("brightness").and_then(|v| v.as_u64()).map(|v| v as u8);
                match brightness {
                    Some(percent) => {
                        let device_id = args.get("device_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let command = smearor_model_macropad::DeviceCommand::SetBrightness(percent);
                        if self.command_sender.send((device_id, command)).is_err() {
                            let response = InvokeToolResponse::error(&message.0.correlation_id, "Command channel closed");
                            broadcaster.broadcast_message_to_topic(response);
                            return;
                        }
                        let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Brightness set to {percent}"));
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: brightness");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            MacroPadMcpTools::LoupedeckSetBrightness => {
                let response = InvokeToolResponse::error(&message.0.correlation_id, "Tool not supported by StreamDeck service");
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}
