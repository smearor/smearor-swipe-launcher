use crate::widget::ButtonWidget;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for ButtonWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, sender_id: &str) {
        let tool_name = format!("button_{}", self.meta.id);
        let requested_name = message.0.name.to_string();
        if requested_name != tool_name && !sender_id.starts_with("web:") {
            return;
        }
        debug!("ButtonWidget: handle_message tool={} correlation_id={}", requested_name, message.0.correlation_id);

        let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
        let action_str = args.get("action").and_then(|v| v.as_str()).unwrap_or("click");

        let action_kind = match smearor_swipe_launcher_plugin_api::ActionKind::from_str(action_str) {
            Ok(kind) => kind,
            Err(_) => {
                let response = InvokeToolResponse::error(&message.0.correlation_id.to_string(), &format!("Unknown action: {action_str}"));
                let broadcaster = self.get_broadcaster();
                broadcaster.broadcast_message_to_topic(response);
                return;
            }
        };

        let response = {
            let broadcaster = self.get_broadcaster();
            if self.config.dispatch_by_kind(action_kind, &broadcaster) {
                debug!("ButtonWidget: dispatched action '{}'", action_kind.as_ref());
                InvokeToolResponse::success(&message.0.correlation_id.to_string(), &format!("Button action '{}' triggered", action_kind.as_ref()))
            } else {
                InvokeToolResponse::error(
                    &message.0.correlation_id.to_string(),
                    &format!("No topic/payload configured for action: {}", action_kind.as_ref()),
                )
            }
        };
        let broadcaster = self.get_broadcaster();
        broadcaster.broadcast_message_to_topic(response);
    }
}
