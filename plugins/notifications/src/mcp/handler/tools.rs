use crate::widget::NotificationView;
use crate::widget::NotificationWidget;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_notifications_model::NotificationWidgetAction;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DefaultFallback;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for NotificationWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let arguments = message.0.arguments.to_string();
        debug!("NotificationWidget: InvokeToolMessage name={} args={}", tool_name, arguments);

        let own_button_name = format!("button_{}", self.meta.id);
        if tool_name == own_button_name {
            let action_str = serde_json::from_str::<serde_json::Value>(&arguments)
                .ok()
                .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
                .unwrap_or_else(|| "click".to_string());

            if let Ok(widget_action) = NotificationWidgetAction::from_str(&action_str) {
                match widget_action {
                    NotificationWidgetAction::Expand => {
                        self.set_view(NotificationView::Expanded);
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "expanded");
                        self.get_broadcaster().broadcast_message_to_topic(response);
                        return;
                    }
                    NotificationWidgetAction::Collapse => {
                        self.set_view(NotificationView::Compact);
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "collapsed");
                        self.get_broadcaster().broadcast_message_to_topic(response);
                        return;
                    }
                    NotificationWidgetAction::ToggleView => {
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
