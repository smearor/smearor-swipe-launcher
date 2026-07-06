use serde::Deserialize;
use serde::Serialize;

/// Available weather views that the widget can display.
/// Each variant corresponds to a data category rendered in the widget tile.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum WeatherView {
    /// Current temperature and cloud cover.
    #[default]
    Current,
    /// Today's forecast: temperature range and cloud cover.
    ForecastToday,
    /// Tomorrow's forecast: temperature range and cloud cover.
    ForecastTomorrow,
    /// Wind speed and direction.
    Wind,
    /// Relative humidity.
    Humidity,
    /// Sunrise and sunset times.
    SunriseSunset,
    /// Air quality index and pollutants.
    AirPollution,
    /// Atmospheric pressure.
    Pressure,
    /// UV index.
    UvIndex,
}
