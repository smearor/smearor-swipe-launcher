/// Current weather conditions snapshot.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct CurrentWeather {
    /// Air temperature at 2 m above ground in degrees Celsius.
    pub temperature: stabby::option::Option<f32>,
    /// Cloud cover percentage (0.0 - 100.0).
    pub cloud_cover: stabby::option::Option<f32>,
    /// Relative humidity percentage (0.0 - 100.0).
    pub relative_humidity: stabby::option::Option<f32>,
    /// Wind speed at 10 m in km/h.
    pub wind_speed: stabby::option::Option<f32>,
    /// Wind direction at 10 m in degrees (0 = North, 180 = South).
    pub wind_direction: stabby::option::Option<f32>,
    /// Surface pressure in hPa.
    pub pressure: stabby::option::Option<f32>,
    /// UV index (0.0 - 11.0+).
    pub uv_index: stabby::option::Option<f32>,
    /// Weather interpretation code (WMO code).
    pub weather_code: stabby::option::Option<u16>,
    /// Whether it is currently day or night.
    pub is_day: stabby::option::Option<bool>,
}

impl CurrentWeather {
    /// Creates a new current weather snapshot.
    pub fn new() -> Self {
        Self::default()
    }
}
