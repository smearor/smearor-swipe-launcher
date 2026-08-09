use crate::widget::WorkspaceSwitcherWidget;
use schemars::schema_for;
use smearor_model_mcp::ButtonActionArgs;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::ToolAnnotations;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

impl McpCapabilitiesRegistrator for WorkspaceSwitcherWidget {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let button_schema = serde_json::to_string(&schema_for!(ButtonActionArgs)).unwrap_or_default();
        let button_tool_name = format!("button_{}", self.meta.id);
        let button_tool = RegisterToolMessage::new(
            &button_tool_name,
            self.config
                .metadata
                .description()
                .unwrap_or("Trigger an action on the workspace switcher widget (view switching or input action)."),
            &button_schema,
        )
        .with_annotations(&ToolAnnotations::idempotent())
        .maybe_with_title(self.config.metadata.title());
        broadcaster.broadcast_message_to_topic(button_tool);
    }
}
