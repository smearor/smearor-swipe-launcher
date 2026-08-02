use crate::widget::WeatherWidget;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DefaultFallback;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_weather_model::WeatherCommandMessage;
use smearor_weather_model::WeatherWidgetAction;
use std::str::FromStr;
use tracing::trace;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for WeatherWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let arguments = message.0.arguments.to_string();
        trace!("weather widget: InvokeToolMessage name={} args={}", tool_name, arguments);

        if tool_name == "weather_widget_refresh" {
            let broadcaster = self.get_broadcaster();
            let command = WeatherCommandMessage::refresh();
            broadcaster.broadcast_message_to_topic(command);
            let response = InvokeToolResponse::success(&message.0.correlation_id, "Refresh triggered");
            broadcaster.broadcast_message_to_topic(response);
            return;
        }

        let own_button_name = format!("button_{}", self.meta.id);
        if tool_name == own_button_name || sender_id.starts_with("web:") {
            let action_str = serde_json::from_str::<serde_json::Value>(&arguments)
                .ok()
                .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
                .unwrap_or_else(|| "click".to_string());

            if let Ok(widget_action) = WeatherWidgetAction::from_str(&action_str) {
                match widget_action {
                    WeatherWidgetAction::Expand => {
                        self.expand_view();
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "expanded");
                        self.get_broadcaster().broadcast_message_to_topic(response);
                        return;
                    }
                    WeatherWidgetAction::Collapse => {
                        self.collapse_view();
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "collapsed");
                        self.get_broadcaster().broadcast_message_to_topic(response);
                        return;
                    }
                    WeatherWidgetAction::ToggleView => {
                        self.toggle_view();
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "view toggled");
                        self.get_broadcaster().broadcast_message_to_topic(response);
                        return;
                    }
                }
            }

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

            let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("{} handled", action_str));
            broadcaster.broadcast_message_to_topic(response);
        }
    }
}
