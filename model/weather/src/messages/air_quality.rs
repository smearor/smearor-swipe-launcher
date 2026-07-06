/// Air quality measurements.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct AirQualityData {
    /// European Air Quality Index (0 - 100).
    pub european_aqi: stabby::option::Option<f32>,
    /// PM10 concentration in micrograms per cubic metre.
    pub pm10: stabby::option::Option<f32>,
    /// PM2.5 concentration in micrograms per cubic metre.
    pub pm2_5: stabby::option::Option<f32>,
    /// Ozone concentration in micrograms per cubic metre.
    pub ozone: stabby::option::Option<f32>,
    /// Nitrogen dioxide concentration in micrograms per cubic metre.
    pub nitrogen_dioxide: stabby::option::Option<f32>,
    /// Sulphur dioxide concentration in micrograms per cubic metre.
    pub sulphur_dioxide: stabby::option::Option<f32>,
    /// Carbon monoxide concentration in micrograms per cubic metre.
    pub carbon_monoxide: stabby::option::Option<f32>,
}

impl AirQualityData {
    /// Creates a new air quality data snapshot.
    pub fn new() -> Self {
        Self::default()
    }
}
