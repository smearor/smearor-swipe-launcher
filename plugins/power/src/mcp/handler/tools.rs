use crate::widget::PowerWidget;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DefaultFallback;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::trace;

/// View actions that can be triggered via MCP tool invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PowerWidgetAction {
    /// Expand to the Confirm view.
    Expand,
    /// Collapse to the Compact view.
    Collapse,
    /// Toggle between Compact and Confirm views.
    ToggleView,
}

impl FromStr for PowerWidgetAction {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "expand" => Ok(Self::Expand),
            "collapse" => Ok(Self::Collapse),
            "toggle_view" => Ok(Self::ToggleView),
            _ => Err(()),
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for PowerWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let arguments = message.0.arguments.to_string();
        trace!("power widget: InvokeToolMessage name={} args={}", tool_name, arguments);

        let own_button_name = format!("button_{}", self.meta.id);
        if tool_name == own_button_name || sender_id.starts_with("web:") {
            let action_str = serde_json::from_str::<serde_json::Value>(&arguments)
                .ok()
                .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
                .unwrap_or_else(|| "click".to_string());

            if let Ok(widget_action) = PowerWidgetAction::from_str(&action_str) {
                match widget_action {
                    PowerWidgetAction::Expand => {
                        self.expand_view();
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "expanded");
                        self.get_broadcaster().broadcast_message_to_topic(response);
                        return;
                    }
                    PowerWidgetAction::Collapse => {
                        self.collapse_view();
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "collapsed");
                        self.get_broadcaster().broadcast_message_to_topic(response);
                        return;
                    }
                    PowerWidgetAction::ToggleView => {
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
