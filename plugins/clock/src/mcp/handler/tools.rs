use crate::widget::ClockWidget;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;
use tracing::trace;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for ClockWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        trace!("clock: handle_message name={}", message.0.name);
        let tool_name = message.0.name.to_string();
        let broadcaster = self.get_broadcaster();

        if tool_name == "get_current_time" {
            let response = if let Some(time_json) = self.clock.get_time_info_json() {
                trace!("clock: get_current_time responding with {}", time_json);
                InvokeToolResponse::success(&message.0.correlation_id.to_string(), &time_json)
            } else {
                debug!("clock: get_current_time not ready");
                InvokeToolResponse::error(&message.0.correlation_id.to_string(), "Clock not ready")
            };
            broadcaster.broadcast_message_to_topic(response);
            return;
        }

        let own_button_name = format!("button_{}", self.meta.id);
        if tool_name == own_button_name {
            let action_str = serde_json::from_str::<serde_json::Value>(&message.0.arguments)
                .ok()
                .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
                .unwrap_or_else(|| "click".to_string());

            let action_kind = ActionKind::from_str(&action_str).ok();

            if let Some(kind) = action_kind {
                let binding = self.config.binding_for_kind(kind);
                if binding.is_configured() {
                    binding.dispatch(&broadcaster);
                }
            }

            let response = InvokeToolResponse::success(&message.0.correlation_id.to_string(), &format!("{} handled", action_str));
            broadcaster.broadcast_message_to_topic(response);
        }
    }
}
