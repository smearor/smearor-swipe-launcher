use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the weather service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeatherMcpTools {
    /// Refresh weather data.
    Refresh,
    /// Get weather forecast for coordinates.
    GetForecast,
    /// Get current location information.
    GetLocation,
    /// Look up coordinates for a place name.
    LookupCoordinates,
    /// Look up a location name for coordinates.
    LookupLocationName,
}

impl AsRef<str> for WeatherMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::Refresh => "weather_refresh",
            Self::GetForecast => "weather_get_forecast",
            Self::GetLocation => "weather_get_location",
            Self::LookupCoordinates => "weather_lookup_coordinates",
            Self::LookupLocationName => "weather_lookup_location_name",
        }
    }
}

impl FromStr for WeatherMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "weather_refresh" => Ok(Self::Refresh),
            "weather_get_forecast" => Ok(Self::GetForecast),
            "weather_get_location" => Ok(Self::GetLocation),
            "weather_lookup_coordinates" => Ok(Self::LookupCoordinates),
            "weather_lookup_location_name" => Ok(Self::LookupLocationName),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for WeatherMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
