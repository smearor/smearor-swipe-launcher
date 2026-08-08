use crate::service::NotificationCommand;
use crate::service::NotificationService;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_notifications_model::NotificationMcpTools;
use smearor_notifications_model::NotificationSendArgs;
use smearor_notifications_model::NotificationToggleDndArgs;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for NotificationService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("Notification Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match NotificationMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &message.0.correlation_id)));
                return;
            }
        };
        match tool {
            NotificationMcpTools::Send => {
                let args: NotificationSendArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                let urgency = args.urgency_level();
                let _ = self.command_sender.send(NotificationCommand::Send {
                    summary: args.summary,
                    body: args.body,
                    urgency,
                });
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Notification sent");
                broadcaster.broadcast_message_to_topic(response);
            }
            NotificationMcpTools::ToggleDnd => {
                let args: NotificationToggleDndArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                let enabled = args.enabled.unwrap_or(true);
                let _ = self.command_sender.send(NotificationCommand::SetDoNotDisturb(enabled));
                let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Do-Not-Disturb set to {enabled}"));
                broadcaster.broadcast_message_to_topic(response);
            }
            NotificationMcpTools::Clear => {
                let _ = self.command_sender.send(NotificationCommand::DismissAll);
                let response = InvokeToolResponse::success(&message.0.correlation_id, "All notifications cleared");
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}
