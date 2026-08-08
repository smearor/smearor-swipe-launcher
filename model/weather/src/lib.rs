#![recursion_limit = "256"]

mod mcp;
mod messages;
mod model;
mod topics;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use mcp::prompts::WeatherMcpPrompts;
pub use mcp::requests::WeatherGetForecastArgs;
pub use mcp::requests::WeatherLookupCoordinatesArgs;
pub use mcp::requests::WeatherLookupLocationNameArgs;
pub use mcp::requests::WeatherQueryGuideArgs;
pub use mcp::resources::WeatherMcpResources;
pub use mcp::tools::WeatherMcpTools;
pub use messages::air_quality::AirQualityData;
pub use messages::command::WeatherCommandAction;
pub use messages::command::WeatherCommandMessage;
pub use messages::current::CurrentWeather;
pub use messages::daily::DailyForecast;
pub use messages::daily::DailyForecastData;
pub use messages::status::WeatherStatusMessage;
pub use messages::view::WeatherView;
pub use messages::view::format_time;
pub use messages::voice::VoiceDescribable;
pub use messages::wmo::WeatherCode;
pub use model::air_quality_level::AirQualityLevel;
pub use model::cloud_cover_level::CloudCoverLevel;
pub use model::humidity_level::HumidityLevel;
pub use model::particulate_matter_level::ParticulateMatterLevel;
pub use model::precipitation_amount_level::PrecipitationAmountLevel;
pub use model::precipitation_intensity::PrecipitationIntensity;
pub use model::precipitation_probability_level::PrecipitationProbabilityLevel;
pub use model::pressure_level::PressureLevel;
pub use model::sunshine_level::SunshineLevel;
pub use model::temperature_level::TemperatureLevel;
pub use model::uv_index_level::UvIndexLevel;
pub use model::wind_direction::WindDirection;
pub use model::wind_speed_level::WindSpeedLevel;
pub use topics::TOPIC_COMMAND;
pub use topics::TOPIC_STATUS;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(WeatherCommandMessageConverter, WeatherCommandMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(WeatherStatusMessageConverter, WeatherStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

/// Register all JSON converter implementations for weather messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    WeatherCommandMessageConverter::register_in_host(context);
    WeatherStatusMessageConverter::register_in_host(context);
}
