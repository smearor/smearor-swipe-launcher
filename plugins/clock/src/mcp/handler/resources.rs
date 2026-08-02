use crate::widget::ClockWidget;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for ClockWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, _sender_id: &str) {
        if message.0.uri.to_string() != "clock://time" {
            return;
        }
        let response = if let Some(time_json) = self.clock.get_time_info_json() {
            InvokeResourceResponse::success(&message.0.correlation_id.to_string(), &time_json)
        } else {
            InvokeResourceResponse::error(&message.0.correlation_id.to_string(), "Clock not ready")
        };
        let broadcaster = self.get_broadcaster();
        broadcaster.broadcast_message_to_topic(response);
    }
}
