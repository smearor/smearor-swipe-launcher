use crate::service::LoupedeckService;
use schemars::schema_for;
use smearor_model_macropad::MacroPadSetBrightnessArgs;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use tracing::debug;

impl McpCapabilitiesRegistrator for LoupedeckService {
    fn register_mcp_capabilities(&self) {
        if !self.config.mcp_enabled {
            debug!("Loupedeck Service: MCP tool registration disabled by config");
            return;
        }

        let broadcaster = self.get_broadcaster();

        let set_brightness_schema = serde_json::to_string(&schema_for!(MacroPadSetBrightnessArgs)).unwrap_or_default();
        let set_brightness_tool = RegisterToolMessage::new(
            "loupedeck_set_brightness",
            "Set the brightness of Loupedeck devices. / Helligkeit der Loupedeck Geräte setzen.",
            &set_brightness_schema,
        );
        broadcaster.broadcast_message_to_topic(set_brightness_tool);
    }
}
