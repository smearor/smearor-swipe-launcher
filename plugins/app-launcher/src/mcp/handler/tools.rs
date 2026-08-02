use crate::widget::AppLauncherWidget;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DefaultFallback;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for AppLauncherWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let own_button_name = format!("button_{}", self.meta.id);
        if tool_name != own_button_name {
            return;
        }
        let action_str = serde_json::from_str::<serde_json::Value>(&message.0.arguments)
            .ok()
            .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
            .unwrap_or_else(|| "click".to_string());

        let action_kind = ActionKind::from_str(&action_str).ok();
        let broadcaster = self.get_broadcaster();

        if let Some(kind) = action_kind {
            let binding = self.config.binding_for_kind(kind);
            if binding.is_configured() {
                binding.dispatch(&broadcaster);
                if binding.is_supplement() {
                    self.default_fallback(&kind, &broadcaster);
                }
            } else {
                self.default_fallback(&kind, &broadcaster);
            }
        }

        let response = InvokeToolResponse::success(&message.0.correlation_id.to_string(), &format!("{} handled", action_str));
        broadcaster.broadcast_message_to_topic(response);
    }
}
