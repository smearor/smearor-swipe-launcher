use serde::Deserialize;

/// Configuration for the weather service.
#[derive(Clone, Debug, Deserialize)]
pub struct WeatherServiceConfig {
    /// Latitude of the location to fetch weather for.
    pub latitude: f64,
    /// Longitude of the location to fetch weather for.
    pub longitude: f64,
    /// Update interval in minutes (minimum 10).
    #[serde(default = "default_update_interval")]
    pub update_interval_minutes: u64,
    /// Whether to fetch air quality data.
    #[serde(default = "default_enable_air_quality")]
    pub enable_air_quality: bool,
    /// Timezone string ("auto" by default).
    #[serde(default = "default_timezone")]
    pub timezone: String,
    /// Number of forecast days to request (1-16).
    #[serde(default = "default_forecast_days")]
    pub forecast_days: u8,
}

impl Default for WeatherServiceConfig {
    fn default() -> Self {
        Self {
            latitude: 52.52,
            longitude: 13.41,
            update_interval_minutes: default_update_interval(),
            enable_air_quality: default_enable_air_quality(),
            timezone: default_timezone(),
            forecast_days: default_forecast_days(),
        }
    }
}

fn default_update_interval() -> u64 {
    30
}

fn default_enable_air_quality() -> bool {
    true
}

fn default_timezone() -> String {
    "auto".to_string()
}

fn default_forecast_days() -> u8 {
    2
}
