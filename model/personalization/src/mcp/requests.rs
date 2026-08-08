use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `set_current_location` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SetCurrentLocationArgs {
    /// Latitude
    pub latitude: f64,
    /// Longitude
    pub longitude: f64,
    /// Optional location name
    pub location_name: Option<String>,
}

/// Arguments for the `set_locale` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SetLocaleArgs {
    /// Locale string like 'de-DE' or 'en-US'
    pub locale: String,
}
