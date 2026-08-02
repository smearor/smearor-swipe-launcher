use crate::widget::WeatherWidget;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for WeatherWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, _sender_id: &str) {
        if message.0.uri != "weather://widget" {
            return;
        }
        let status = self.latest_status.borrow().clone();
        let response = match status {
            Some(status) => {
                let json = serde_json::json!({
                    "latitude": status.latitude,
                    "longitude": status.longitude,
                    "success": status.success,
                    "is_stale": status.is_stale,
                    "last_updated": status.last_updated.to_string(),
                });
                InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
            }
            None => InvokeResourceResponse::error(&message.0.correlation_id, "No weather data available"),
        };
        let broadcaster = self.get_broadcaster();
        broadcaster.broadcast_message_to_topic(response);
    }
}
