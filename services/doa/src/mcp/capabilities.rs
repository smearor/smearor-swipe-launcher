use crate::service::DoaService;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use tracing::debug;

impl McpCapabilitiesRegistrator for DoaService {
    fn register_mcp_capabilities(&self) {
        if !self.config.mcp_enabled {
            debug!("DoA Service: MCP tool registration disabled by config");
            return;
        }

        let broadcaster = self.get_broadcaster();

        let doa_resource = RegisterResourceMessage::new(
            "doa://status",
            "DoA Sensor Status",
            "Current Direction of Arrival angle, mapped direction, and device connection status.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(doa_resource);

        let get_direction_tool = RegisterToolMessage::new(
            "doa_get_direction",
            "Returns the current DoA angle (0-359), mapped compass direction (N/E/S/W), and device connection status.",
            r#"{ "type": "object", "properties": {}, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(get_direction_tool);

        let set_poll_interval_tool = RegisterToolMessage::new(
            "doa_set_poll_interval",
            "Sets the DoA polling interval in milliseconds. Lower values give more responsive direction updates but increase USB traffic. Minimum: 50ms.",
            r#"{ "type": "object", "properties": { "interval_ms": { "type": "integer", "description": "Polling interval in milliseconds (min: 50, default: 150)" } }, "required": ["interval_ms"] }"#,
        );
        broadcaster.broadcast_message_to_topic(set_poll_interval_tool);

        let reconnect_tool = RegisterToolMessage::new(
            "doa_reconnect",
            "Forces a USB reconnection to the ReSpeaker XVF3800 device. Use this if the device was unplugged and reconnected.",
            r#"{ "type": "object", "properties": {}, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(reconnect_tool);

        let prompt = RegisterPromptMessage::with_memory(
            "doa_guide",
            "Returns a system prompt with DoA sensor tools, resources, and current direction snapshot.",
            r#"{ "type": "object", "properties": {} }"#,
            "DoA sensor preferences and polling interval settings",
            "doa,direction,sensor,microphone",
        );
        broadcaster.broadcast_message_to_topic(prompt);
    }
}
