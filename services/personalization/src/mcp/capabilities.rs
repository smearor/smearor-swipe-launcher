use crate::service::PersonalizationService;
use schemars::schema_for;
use smearor_model_mcp::NoArgs;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_personalization_model::SetCurrentLocationArgs;
use smearor_personalization_model::SetLocaleArgs;
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

        let no_args_schema = serde_json::to_string(&schema_for!(NoArgs)).unwrap_or_default();
        let get_location_tool =
            RegisterToolMessage::new("get_current_location", "Returns the user's current latitude, longitude, and location name.", &no_args_schema);
        broadcaster.broadcast_message_to_topic(get_location_tool);

        let get_timezone_tool =
            RegisterToolMessage::new("get_timezone", "Returns the current IANA timezone identifier (e.g. 'Europe/Berlin').", &no_args_schema);
        broadcaster.broadcast_message_to_topic(get_timezone_tool);

        let get_locale_tool = RegisterToolMessage::new("get_locale", "Returns the current system locale string (e.g. 'de-DE', 'en-US').", &no_args_schema);
        broadcaster.broadcast_message_to_topic(get_locale_tool);

        let get_personalization_tool = RegisterToolMessage::new("get_personalization", "Returns the full personalization profile as JSON.", &no_args_schema);
        broadcaster.broadcast_message_to_topic(get_personalization_tool);

        let set_location_schema = serde_json::to_string(&schema_for!(SetCurrentLocationArgs)).unwrap_or_default();
        let set_location_tool = RegisterToolMessage::new(
            "set_current_location",
            "Sets a runtime override for the user's location. Accepts latitude and longitude. The override persists until a refresh is triggered.",
            &set_location_schema,
        );
        broadcaster.broadcast_message_to_topic(set_location_tool);

        let set_locale_schema = serde_json::to_string(&schema_for!(SetLocaleArgs)).unwrap_or_default();
        let set_locale_tool = RegisterToolMessage::new(
            "set_locale",
            "Sets a runtime override for the user's locale. Accepts a locale string like 'de-DE' or 'en-US'. The override persists until a refresh is triggered.",
            &set_locale_schema,
        );
        broadcaster.broadcast_message_to_topic(set_locale_tool);

        let refresh_tool = RegisterToolMessage::new(
            "refresh_personalization",
            "Clears all runtime overrides and re-queries system APIs for personalization data.",
            &no_args_schema,
        );
        broadcaster.broadcast_message_to_topic(refresh_tool);

        let prompt = RegisterPromptMessage::with_memory(
            "personalization_guide",
            "Returns a system prompt with personalization tools, resources, and current profile snapshot.",
            &no_args_schema,
            "personalization preferences including locale, timezone, and location",
            "personalization,locale,timezone,location",
        );
        broadcaster.broadcast_message_to_topic(prompt);
    }
}
