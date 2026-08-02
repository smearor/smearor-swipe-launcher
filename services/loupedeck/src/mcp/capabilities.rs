use crate::service::LoupedeckService;
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

        let set_brightness_tool = RegisterToolMessage::new(
            "loupedeck_set_brightness",
            "Set the brightness of Loupedeck devices. / Helligkeit der Loupedeck Geräte setzen.",
            r#"{ "type": "object", "properties": { "brightness": { "type": "integer", "minimum": 0, "maximum": 100, "description": "Brightness percentage (0-100)" }, "device_id": { "type": "string", "description": "Device serial number (empty = all connected Loupedeck devices)" } }, "required": ["brightness"] }"#,
        );
        broadcaster.broadcast_message_to_topic(set_brightness_tool);
    }
}
