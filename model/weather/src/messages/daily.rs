/// Forecast data for a single day.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct DailyForecast {
    /// Maximum air temperature at 2 m in degrees Celsius.
    pub temperature_max: stabby::option::Option<f32>,
    /// Minimum air temperature at 2 m in degrees Celsius.
    pub temperature_min: stabby::option::Option<f32>,
    /// Cloud cover percentage (0.0 - 100.0).
    pub cloud_cover: stabby::option::Option<f32>,
    /// Sunrise time as ISO-8601 string.
    pub sunrise: stabby::option::Option<stabby::string::String>,
    /// Sunset time as ISO-8601 string.
    pub sunset: stabby::option::Option<stabby::string::String>,
    /// UV index max for the day.
    pub uv_index_max: stabby::option::Option<f32>,
    /// Maximum wind speed in km/h.
    pub wind_speed_max: stabby::option::Option<f32>,
    /// Precipitation sum in mm.
    pub precipitation_sum: stabby::option::Option<f32>,
    /// Weather interpretation code (WMO code).
    pub weather_code: stabby::option::Option<u16>,
}

impl DailyForecast {
    /// Creates a new daily forecast.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Container for today's and tomorrow's daily forecast.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct DailyForecastData {
    /// Forecast for the current day.
    pub today: DailyForecast,
    /// Forecast for the next day.
    pub tomorrow: DailyForecast,
}
