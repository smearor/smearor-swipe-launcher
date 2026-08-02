use crate::service::StreamDeckService;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use tracing::debug;

impl McpCapabilitiesRegistrator for StreamDeckService {
    fn register_mcp_capabilities(&self) {
        if !self.config.mcp_enabled {
            debug!("StreamDeck Service: MCP tool registration disabled by config");
            return;
        }

        let broadcaster = self.get_broadcaster();

        let set_brightness_tool = RegisterToolMessage::new(
            "streamdeck_set_brightness",
            "Set the brightness of Stream Deck devices. / Helligkeit der Stream Deck Geräte setzen.",
            r#"{ "type": "object", "properties": { "brightness": { "type": "integer", "minimum": 0, "maximum": 100, "description": "Brightness percentage (0-100)" }, "device_id": { "type": "string", "description": "Device serial number (empty = all connected Stream Deck devices)" } }, "required": ["brightness"] }"#,
        );
        broadcaster.broadcast_message_to_topic(set_brightness_tool);
    }
}
