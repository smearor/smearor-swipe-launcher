use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use stabby::option::Option as StabbyOption;

use crate::AirQualityData;
use crate::CurrentWeather;
use crate::DailyForecast;
use crate::DailyForecastData;
use crate::WeatherCommandAction;
use crate::WeatherCommandMessage;
use crate::WeatherStatusMessage;

fn parse_f32(value: &serde_json::Value, field: &str) -> StabbyOption<f32> {
    value
        .get(field)
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None())
}

fn parse_u16(value: &serde_json::Value, field: &str) -> StabbyOption<u16> {
    value
        .get(field)
        .and_then(|v| v.as_u64())
        .map(|v| v as u16)
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None())
}

fn parse_bool(value: &serde_json::Value, field: &str) -> StabbyOption<bool> {
    value
        .get(field)
        .and_then(|v| v.as_bool())
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None())
}

fn parse_string(value: &serde_json::Value, field: &str) -> StabbyOption<stabby::string::String> {
    value
        .get(field)
        .and_then(|v| if v.is_null() { None } else { Some(v.as_str().unwrap_or("")) })
        .map(stabby::string::String::from)
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None())
}

fn parse_current_weather(value: &serde_json::Value) -> CurrentWeather {
    CurrentWeather {
        temperature: parse_f32(value, "temperature"),
        cloud_cover: parse_f32(value, "cloud_cover"),
        relative_humidity: parse_f32(value, "relative_humidity"),
        wind_speed: parse_f32(value, "wind_speed"),
        wind_direction: parse_f32(value, "wind_direction"),
        pressure: parse_f32(value, "pressure"),
        uv_index: parse_f32(value, "uv_index"),
        weather_code: parse_u16(value, "weather_code"),
        is_day: parse_bool(value, "is_day"),
    }
}

fn parse_daily_forecast(value: &serde_json::Value) -> DailyForecast {
    DailyForecast {
        temperature_max: parse_f32(value, "temperature_max"),
        temperature_min: parse_f32(value, "temperature_min"),
        cloud_cover: parse_f32(value, "cloud_cover"),
        sunrise: parse_string(value, "sunrise"),
        sunset: parse_string(value, "sunset"),
        uv_index_max: parse_f32(value, "uv_index_max"),
        wind_speed_max: parse_f32(value, "wind_speed_max"),
        precipitation_sum: parse_f32(value, "precipitation_sum"),
        weather_code: parse_u16(value, "weather_code"),
    }
}

fn parse_daily_forecast_data(value: &serde_json::Value) -> DailyForecastData {
    DailyForecastData {
        today: value.get("today").map(parse_daily_forecast).unwrap_or_default(),
        tomorrow: value.get("tomorrow").map(parse_daily_forecast).unwrap_or_default(),
    }
}

fn parse_air_quality(value: &serde_json::Value) -> AirQualityData {
    AirQualityData {
        european_aqi: parse_f32(value, "european_aqi"),
        pm10: parse_f32(value, "pm10"),
        pm2_5: parse_f32(value, "pm2_5"),
        ozone: parse_f32(value, "ozone"),
        nitrogen_dioxide: parse_f32(value, "nitrogen_dioxide"),
        sulphur_dioxide: parse_f32(value, "sulphur_dioxide"),
        carbon_monoxide: parse_f32(value, "carbon_monoxide"),
    }
}

smearor_swipe_launcher_plugin_api::impl_json_convertible!(WeatherCommandMessageConverter, WeatherCommandMessage, |json: serde_json::Value| {
    let action = match json.get("action").and_then(|v| v.as_str()) {
        Some("Refresh") => WeatherCommandAction::Refresh,
        _ => WeatherCommandAction::Refresh,
    };
    WeatherCommandMessage::new(action)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(WeatherStatusMessageConverter, WeatherStatusMessage, |json: serde_json::Value| {
    let latitude = json.get("latitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let longitude = json.get("longitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let current = json.get("current").map(parse_current_weather).unwrap_or_default();
    let daily = json.get("daily").map(parse_daily_forecast_data).unwrap_or_default();
    let air_quality = json
        .get("air_quality")
        .filter(|v| !v.is_null())
        .map(|v| StabbyOption::Some(parse_air_quality(v)))
        .unwrap_or(StabbyOption::None());
    let last_updated = json
        .get("last_updated")
        .and_then(|v| v.as_str())
        .map(stabby::string::String::from)
        .unwrap_or_default();
    let success = json.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    let is_stale = json.get("is_stale").and_then(|v| v.as_bool()).unwrap_or(false);
    let error_message = parse_string(&json, "error_message");
    WeatherStatusMessage {
        latitude,
        longitude,
        current,
        daily,
        air_quality,
        last_updated,
        success,
        is_stale,
        error_message,
    }
});

/// Register all JSON converter implementations for weather messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    WeatherCommandMessageConverter::register_in_host(context);
    WeatherStatusMessageConverter::register_in_host(context);
}
