use crate::widget::AppLauncherWidget;
use schemars::schema_for;
use smearor_model_mcp::ButtonActionArgs;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::ToolAnnotations;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

impl McpCapabilitiesRegistrator for AppLauncherWidget {
    fn register_mcp_capabilities(&self) {
        let schema = serde_json::to_string(&schema_for!(ButtonActionArgs)).unwrap_or_default();
        let tool = RegisterToolMessage::new(
            &format!("button_{}", self.meta.id),
            self.config.metadata.description().unwrap_or("App launcher widget"),
            &schema,
        )
        .with_annotations(&ToolAnnotations::idempotent().with_open_world(true))
        .maybe_with_title(self.config.metadata.title());
        MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(tool);
    }
}
