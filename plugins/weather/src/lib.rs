pub(crate) mod atomic;
pub(crate) mod config;
pub(crate) mod graphic;
pub(crate) mod html;
pub(crate) mod labels;
pub(crate) mod mcp;
pub(crate) mod personalization;
pub(crate) mod widget;

use crate::atomic::WeatherAtomicWidget;
use crate::widget::WeatherWidget;
use smearor_swipe_launcher_plugin_api::widget_factory_plugin_graphic;

widget_factory_plugin_graphic! {
    "weather" => weather_widget => WeatherWidget => html,
    "weather_today" => weather_today_widget => WeatherAtomicWidget,
    "weather_forecast" => weather_forecast_widget => WeatherAtomicWidget,
    "weather_tomorrow" => weather_tomorrow_widget => WeatherAtomicWidget,
    "weather_uv_index" => weather_uv_index_widget => WeatherAtomicWidget,
    "weather_sunrise" => weather_sunrise_widget => WeatherAtomicWidget,
    "weather_sunset" => weather_sunset_widget => WeatherAtomicWidget,
    "weather_cloud_cover" => weather_cloud_cover_widget => WeatherAtomicWidget,
    "weather_sunshine" => weather_sunshine_widget => WeatherAtomicWidget,
    "weather_precipitation_probability" => weather_precipitation_probability_widget => WeatherAtomicWidget,
    "weather_precipitation_amount" => weather_precipitation_amount_widget => WeatherAtomicWidget,
    "weather_precipitation" => weather_precipitation_widget => WeatherAtomicWidget,
    "weather_wind" => weather_wind_widget => WeatherAtomicWidget,
    "weather_humidity" => weather_humidity_widget => WeatherAtomicWidget,
    "weather_air_pollution" => weather_air_pollution_widget => WeatherAtomicWidget,
    "weather_pressure" => weather_pressure_widget => WeatherAtomicWidget,
}
