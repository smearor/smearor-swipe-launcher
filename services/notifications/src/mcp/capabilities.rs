use crate::service::NotificationService;
use schemars::schema_for;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::ToolAnnotations;
use smearor_notifications_model::NotificationClearArgs;
use smearor_notifications_model::NotificationSendArgs;
use smearor_notifications_model::NotificationToggleDndArgs;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

impl McpCapabilitiesRegistrator for NotificationService {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let history_resource = RegisterResourceMessage::new(
            "notifications://history",
            "Notification History",
            "List of all active/recent notifications with app name, summary, body, urgency, and timestamp.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(history_resource);

        let dnd_resource =
            RegisterResourceMessage::new("notifications://dnd", "Do Not Disturb Status", "Current Do-Not-Disturb mode status.", "application/json");
        broadcaster.broadcast_message_to_topic(dnd_resource);

        let send_schema = serde_json::to_string(&schema_for!(NotificationSendArgs)).unwrap_or_default();
        let send_tool =
            RegisterToolMessage::new("notifications_send", "Send a desktop notification.", &send_schema).with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(send_tool);

        let toggle_dnd_schema = serde_json::to_string(&schema_for!(NotificationToggleDndArgs)).unwrap_or_default();
        let toggle_dnd_tool = RegisterToolMessage::new("notifications_toggle_dnd", "Toggle Do-Not-Disturb mode on or off.", &toggle_dnd_schema)
            .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(toggle_dnd_tool);

        let clear_schema = serde_json::to_string(&schema_for!(NotificationClearArgs)).unwrap_or_default();
        let clear_tool = RegisterToolMessage::new("notifications_clear", "Dismiss all active notifications.", &clear_schema)
            .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(clear_tool);
    }
}
