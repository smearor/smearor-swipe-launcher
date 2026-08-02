use crate::service::PowerCommand;
use crate::service::PowerService;
use crate::service::parse_action_from_string;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_power_model::PowerAction;
use smearor_power_model::PowerMcpTools;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::trace;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for PowerService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        trace!("Power Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match PowerMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &message.0.correlation_id)));
                return;
            }
        };
        match tool {
            PowerMcpTools::PowerAction => {
                let action_str = serde_json::from_str::<serde_json::Value>(&message.0.arguments.to_string())
                    .ok()
                    .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
                    .unwrap_or_default();
                let power_action = parse_action_from_string(&action_str);
                let _ = self.command_sender.send(PowerCommand::Execute(power_action));
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Power action triggered");
                broadcaster.broadcast_message_to_topic(response);
            }
            PowerMcpTools::SchedulePowerAction => {
                let parsed = serde_json::from_str::<serde_json::Value>(&message.0.arguments.to_string()).ok();
                let action_str = parsed
                    .as_ref()
                    .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
                    .unwrap_or_default();
                let delay_minutes = parsed.as_ref().and_then(|v| v.get("delay_minutes").and_then(|a| a.as_u64())).unwrap_or(0);
                let power_action = parse_action_from_string(&action_str);
                let _ = self.command_sender.send(PowerCommand::Schedule(power_action, delay_minutes));
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Power action scheduled");
                broadcaster.broadcast_message_to_topic(response);
            }
            PowerMcpTools::CancelPowerAction => {
                let _ = self.command_sender.send(PowerCommand::Cancel);
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Power action cancelled");
                broadcaster.broadcast_message_to_topic(response);
            }
            PowerMcpTools::RebootToUefi => {
                let _ = self.command_sender.send(PowerCommand::Execute(PowerAction::RebootToFirmware));
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Reboot to UEFI triggered");
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}
