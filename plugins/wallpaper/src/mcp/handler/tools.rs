use crate::widget::WallpaperWidget;
use smearor_model_mcp::ButtonActionArgs;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::DefaultFallback;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use tracing::trace;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for WallpaperWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let arguments = message.0.arguments.to_string();
        trace!("wallpaper widget: InvokeToolMessage name={} args={}", tool_name, arguments);

        let own_button_name = format!("button_{}", self.meta.id);
        if tool_name != own_button_name && !sender_id.starts_with("web:") {
            return;
        }

        let args: ButtonActionArgs = serde_json::from_str(&arguments).unwrap_or_default();
        let action_kind = args.action;
        let action_str = action_kind.as_ref().to_string();

        let broadcaster = self.get_broadcaster();

        let binding = self.config.binding_for_kind(action_kind);
        if binding.is_configured() {
            binding.dispatch(&broadcaster);
            if binding.is_supplement() {
                self.default_fallback(&action_kind, &broadcaster);
            }
        } else {
            self.default_fallback(&action_kind, &broadcaster);
        }

        let response = InvokeToolResponse::success(&message.0.correlation_id.to_string(), &format!("{} handled", action_str));
        broadcaster.broadcast_message_to_topic(response);
    }
}
