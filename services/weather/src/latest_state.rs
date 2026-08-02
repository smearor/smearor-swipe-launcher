use smearor_weather_model::WeatherStatusMessage;

/// Latest weather state shared between the update loop and MCP handlers.
#[derive(Clone, Default)]
pub struct LatestWeatherState {
    /// Last successful weather status message.
    pub status: WeatherStatusMessage,
}
