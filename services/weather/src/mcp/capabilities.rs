use crate::service::WeatherService;
use schemars::schema_for;
use smearor_model_mcp::NoArgs;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_weather_model::WeatherGetForecastArgs;
use smearor_weather_model::WeatherLookupCoordinatesArgs;
use smearor_weather_model::WeatherLookupLocationNameArgs;
use smearor_weather_model::WeatherQueryGuideArgs;

impl McpCapabilitiesRegistrator for WeatherService {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let resource = RegisterResourceMessage::new(
            "weather://now_at_current_location",
            "Current Weather",
            "Current weather conditions for the configured home location only. Does not support query parameters or custom coordinates. For weather at arbitrary locations, use the weather_get_forecast tool.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(resource);

        let no_args_schema = serde_json::to_string(&schema_for!(NoArgs)).unwrap_or_default();
        let tool = RegisterToolMessage::new("weather_refresh", "Force an immediate refresh of weather data.", &no_args_schema);
        broadcaster.broadcast_message_to_topic(tool);

        let forecast_schema = serde_json::to_string(&schema_for!(WeatherGetForecastArgs)).unwrap_or_default();
        let forecast_tool = RegisterToolMessage::new(
            "weather_get_forecast",
            "Get current weather conditions and forecast for the configured location or arbitrary coordinates. Uses the configured location when no coordinates are provided. Supports custom latitude and longitude for any city.",
            &forecast_schema,
        );
        broadcaster.broadcast_message_to_topic(forecast_tool);

        let location_tool = RegisterToolMessage::new(
            "weather_get_location",
            "Get the configured weather location (latitude, longitude, timezone).",
            &no_args_schema,
        );
        broadcaster.broadcast_message_to_topic(location_tool);

        let lookup_coordinates_schema = serde_json::to_string(&schema_for!(WeatherLookupCoordinatesArgs)).unwrap_or_default();
        let lookup_coordinates_tool = RegisterToolMessage::new(
            "weather_lookup_coordinates",
            "Resolve a place name to latitude and longitude coordinates. Use the returned latitude and longitude as parameters for the 'weather_get_forecast' tool to get the weather for that location.",
            &lookup_coordinates_schema,
        );
        broadcaster.broadcast_message_to_topic(lookup_coordinates_tool);

        let lookup_location_name_schema = serde_json::to_string(&schema_for!(WeatherLookupLocationNameArgs)).unwrap_or_default();
        let lookup_location_name_tool = RegisterToolMessage::new(
            "weather_lookup_location_name",
            "Resolve latitude and longitude coordinates to a human-readable location name.",
            &lookup_location_name_schema,
        );
        broadcaster.broadcast_message_to_topic(lookup_location_name_tool);

        let query_guide_schema = serde_json::to_string(&schema_for!(WeatherQueryGuideArgs)).unwrap_or_default();
        let prompt = RegisterPromptMessage::with_memory(
            "weather_query_guide",
            "Returns a system prompt with weather query instructions and the configured location.",
            &query_guide_schema,
            "weather location preference and temperature unit preference",
            "weather",
        );
        broadcaster.broadcast_message_to_topic(prompt);

        let context_prompt = RegisterPromptMessage::with_memory(
            "weather_context_guide",
            "Returns instructions for resolving weather locations and using the correct coordinates.",
            &no_args_schema,
            "weather location preference and default coordinates",
            "",
        );
        broadcaster.broadcast_message_to_topic(context_prompt);
    }
}
