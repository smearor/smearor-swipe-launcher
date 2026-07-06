use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::AirQualityData;
use crate::CurrentWeather;
use crate::DailyForecastData;
use crate::TOPIC_STATUS;

/// Complete weather status message broadcast by the service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct WeatherStatusMessage {
    /// Latitude of the configured location.
    pub latitude: f64,
    /// Longitude of the configured location.
    pub longitude: f64,
    /// Current weather conditions.
    pub current: CurrentWeather,
    /// Daily forecast for today and tomorrow.
    pub daily: DailyForecastData,
    /// Air quality data.
    pub air_quality: stabby::option::Option<AirQualityData>,
    /// Timestamp of the last successful update as ISO-8601 string.
    pub last_updated: stabby::string::String,
    /// Whether the last update succeeded.
    pub success: bool,
    /// Whether the data is stale (last fetch failed, but cached data is retained).
    pub is_stale: bool,
    /// Error message when the last update failed.
    pub error_message: stabby::option::Option<stabby::string::String>,
}

impl WeatherStatusMessage {
    /// Creates a new weather status message.
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            ..Default::default()
        }
    }
}

impl TypedMessage for WeatherStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_weather_model::WeatherStatusMessage");
}

impl MessageTopic for WeatherStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl SharedMessage for WeatherStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_STATUS
    }
}
