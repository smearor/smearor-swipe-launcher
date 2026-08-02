use crate::service::PersonalizationService;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

impl McpCapabilitiesRegistrator for PersonalizationService {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let resource = RegisterResourceMessage::new(
            "personalization://profile",
            "Personalization Profile",
            "Full personalization profile including location, timezone, locale, and unit/format preferences.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(resource);

        let get_location_tool = RegisterToolMessage::new(
            "get_current_location",
            "Returns the user's current latitude, longitude, and location name.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(get_location_tool);

        let get_timezone_tool = RegisterToolMessage::new(
            "get_timezone",
            "Returns the current IANA timezone identifier (e.g. 'Europe/Berlin').",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(get_timezone_tool);

        let get_locale_tool = RegisterToolMessage::new(
            "get_locale",
            "Returns the current system locale string (e.g. 'de-DE', 'en-US').",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(get_locale_tool);

        let get_personalization_tool = RegisterToolMessage::new(
            "get_personalization",
            "Returns the full personalization profile as JSON.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(get_personalization_tool);

        let set_location_tool = RegisterToolMessage::new(
            "set_current_location",
            "Sets a runtime override for the user's location. Accepts latitude and longitude. The override persists until a refresh is triggered.",
            r#"{ "type": "object", "properties": { "latitude": { "type": "number" }, "longitude": { "type": "number" }, "location_name": { "type": "string" } }, "required": ["latitude", "longitude"] }"#,
        );
        broadcaster.broadcast_message_to_topic(set_location_tool);

        let set_locale_tool = RegisterToolMessage::new(
            "set_locale",
            "Sets a runtime override for the user's locale. Accepts a locale string like 'de-DE' or 'en-US'. The override persists until a refresh is triggered.",
            r#"{ "type": "object", "properties": { "locale": { "type": "string" } }, "required": ["locale"] }"#,
        );
        broadcaster.broadcast_message_to_topic(set_locale_tool);

        let refresh_tool = RegisterToolMessage::new(
            "refresh_personalization",
            "Clears all runtime overrides and re-queries system APIs for personalization data.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(refresh_tool);
    }
}
