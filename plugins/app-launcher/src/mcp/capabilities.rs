use crate::widget::AppLauncherWidget;
use schemars::schema_for;
use smearor_model_mcp::ButtonActionArgs;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

impl McpCapabilitiesRegistrator for AppLauncherWidget {
    fn register_mcp_capabilities(&self) {
        if self.config.description.is_some() {
            let schema = serde_json::to_string(&schema_for!(ButtonActionArgs)).unwrap_or_default();
            let tool = RegisterToolMessage::new(
                &format!("button_{}", self.meta.id),
                self.config.description.as_deref().unwrap_or("App launcher widget"),
                &schema,
            );
            MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(tool);
        }
    }
}
