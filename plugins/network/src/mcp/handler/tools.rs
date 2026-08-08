use crate::widget::NetworkWidget;
use smearor_model_mcp::ButtonActionArgs;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::DefaultFallback;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for NetworkWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let own_button_name = format!("button_{}", self.meta.id);
        if tool_name != own_button_name {
            return;
        }
        let args: ButtonActionArgs = serde_json::from_str(&message.0.arguments).unwrap_or_default();
        let action_kind = args.action;
        let action_str = action_kind.as_ref().to_string();

        let broadcaster = self.get_broadcaster();

        let binding = self.config.binding_for_kind(action_kind);
        binding.dispatch_with_fallback(&broadcaster, Box::new(|| self.default_fallback(&action_kind, &broadcaster)));

        let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("{} handled", action_str));
        broadcaster.broadcast_message_to_topic(response);
    }
}
