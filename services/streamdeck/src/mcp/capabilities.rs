use crate::service::StreamDeckService;
use schemars::schema_for;
use smearor_model_macropad::MacroPadSetBrightnessArgs;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::ToolAnnotations;
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

        let schema = serde_json::to_string(&schema_for!(MacroPadSetBrightnessArgs)).unwrap_or_default();
        let set_brightness_tool = RegisterToolMessage::new(
            "streamdeck_set_brightness",
            "Set the brightness of Stream Deck devices. / Helligkeit der Stream Deck Geräte setzen.",
            &schema,
        )
        .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(set_brightness_tool);
    }
}
