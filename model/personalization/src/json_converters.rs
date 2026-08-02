use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use stabby::option::Option as StabbyOption;

use crate::ColorScheme;
use crate::DateFormat;
use crate::FirstDayOfWeek;
use crate::GeoCoordinates;
use crate::MeasurementSystem;
use crate::PersonalizationCommandAction;
use crate::PersonalizationCommandMessage;
use crate::PersonalizationStatusMessage;
use crate::TemperatureUnit;
use crate::TimeFormat;
use crate::WindSpeedUnit;

fn parse_string(value: &serde_json::Value, field: &str) -> StabbyOption<stabby::string::String> {
    value
        .get(field)
        .and_then(|v| if v.is_null() { None } else { Some(v.as_str().unwrap_or("")) })
        .map(stabby::string::String::from)
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None())
}

fn parse_coordinates(value: &serde_json::Value) -> StabbyOption<GeoCoordinates> {
    value
        .get("coordinates")
        .filter(|v| !v.is_null())
        .map(|v| GeoCoordinates {
            latitude: v.get("latitude").and_then(|f| f.as_f64()).unwrap_or(0.0),
            longitude: v.get("longitude").and_then(|f| f.as_f64()).unwrap_or(0.0),
            location_name: parse_string(v, "location_name"),
        })
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None())
}

fn parse_temperature_unit(value: &serde_json::Value) -> TemperatureUnit {
    match value.get("temperature_unit").and_then(|v| v.as_str()) {
        Some("Fahrenheit") => TemperatureUnit::Fahrenheit,
        _ => TemperatureUnit::Celsius,
    }
}

fn parse_wind_speed_unit(value: &serde_json::Value) -> WindSpeedUnit {
    match value.get("wind_speed_unit").and_then(|v| v.as_str()) {
        Some("Mph") => WindSpeedUnit::Mph,
        Some("Ms") => WindSpeedUnit::Ms,
        _ => WindSpeedUnit::Kmh,
    }
}

fn parse_time_format(value: &serde_json::Value) -> TimeFormat {
    match value.get("time_format").and_then(|v| v.as_str()) {
        Some("Hour12") => TimeFormat::Hour12,
        _ => TimeFormat::Hour24,
    }
}

fn parse_date_format(value: &serde_json::Value) -> DateFormat {
    match value.get("date_format").and_then(|v| v.as_str()) {
        Some("Mdy") => DateFormat::Mdy,
        Some("Ymd") => DateFormat::Ymd,
        _ => DateFormat::Dmy,
    }
}

fn parse_first_day_of_week(value: &serde_json::Value) -> FirstDayOfWeek {
    match value.get("first_day_of_week").and_then(|v| v.as_str()) {
        Some("Sunday") => FirstDayOfWeek::Sunday,
        _ => FirstDayOfWeek::Monday,
    }
}

fn parse_measurement_system(value: &serde_json::Value) -> MeasurementSystem {
    match value.get("measurement_system").and_then(|v| v.as_str()) {
        Some("Imperial") => MeasurementSystem::Imperial,
        _ => MeasurementSystem::Metric,
    }
}

fn parse_color_scheme(value: &serde_json::Value) -> ColorScheme {
    match value.get("color_scheme").and_then(|v| v.as_str()) {
        Some("Light") => ColorScheme::Light,
        Some("Dark") => ColorScheme::Dark,
        _ => ColorScheme::System,
    }
}

smearor_swipe_launcher_plugin_api::impl_json_convertible!(PersonalizationCommandMessageConverter, PersonalizationCommandMessage, |json: serde_json::Value| {
    let action = match json.get("action").and_then(|v| v.as_str()) {
        Some("UpdateLocation") => {
            let coords_json = json.get("coordinates").unwrap_or(&serde_json::Value::Null);
            let coordinates = GeoCoordinates {
                latitude: coords_json.get("latitude").and_then(|f| f.as_f64()).unwrap_or(0.0),
                longitude: coords_json.get("longitude").and_then(|f| f.as_f64()).unwrap_or(0.0),
                location_name: parse_string(coords_json, "location_name"),
            };
            PersonalizationCommandAction::UpdateLocation(coordinates)
        }
        Some("UpdateLocale") => {
            let locale = json.get("locale").and_then(|v| v.as_str()).unwrap_or("").to_string();
            PersonalizationCommandAction::UpdateLocale(locale.into())
        }
        _ => PersonalizationCommandAction::Refresh,
    };
    PersonalizationCommandMessage::new(action)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(PersonalizationStatusMessageConverter, PersonalizationStatusMessage, |json: serde_json::Value| {
    PersonalizationStatusMessage {
        coordinates: parse_coordinates(&json),
        timezone: parse_string(&json, "timezone"),
        locale: parse_string(&json, "locale"),
        temperature_unit: parse_temperature_unit(&json),
        wind_speed_unit: parse_wind_speed_unit(&json),
        time_format: parse_time_format(&json),
        date_format: parse_date_format(&json),
        first_day_of_week: parse_first_day_of_week(&json),
        measurement_system: parse_measurement_system(&json),
        color_scheme: parse_color_scheme(&json),
        success: json.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
        error_message: parse_string(&json, "error_message"),
    }
});

/// Register all JSON converter implementations for personalization messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    PersonalizationCommandMessageConverter::register_in_host(context);
    PersonalizationStatusMessageConverter::register_in_host(context);
}
