use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `weather_get_forecast` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct WeatherGetForecastArgs {
    /// Latitude for custom coordinates
    pub latitude: Option<f64>,
    /// Longitude for custom coordinates
    pub longitude: Option<f64>,
}

/// Arguments for the `weather_lookup_coordinates` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct WeatherLookupCoordinatesArgs {
    /// Name of the place or city to geocode
    pub place_name: String,
}

/// Arguments for the `weather_lookup_location_name` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct WeatherLookupLocationNameArgs {
    /// Latitude coordinate
    pub latitude: f64,
    /// Longitude coordinate
    pub longitude: f64,
}

/// Arguments for the `weather_query_guide` MCP prompt.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct WeatherQueryGuideArgs {
    /// Whether to include forecast instructions
    pub include_forecast: Option<bool>,
}
