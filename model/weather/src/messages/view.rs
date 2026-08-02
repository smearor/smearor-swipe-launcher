use serde::Deserialize;
use serde::Serialize;

/// Extract the `hh:mm` portion from an ISO-8601 datetime string.
///
/// Returns `"--:--"` if the input is empty or does not contain a time component.
pub fn format_time(iso: &str) -> String {
    if let Some(time_part) = iso.split('T').nth(1) {
        if let Some(hhmm) = time_part.get(..5) {
            return hhmm.to_string();
        }
    }
    "--:--".to_string()
}

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
    /// Sunrise time.
    Sunrise,
    /// Sunset time.
    Sunset,
    /// Air quality index and pollutants.
    AirPollution,
    /// Atmospheric pressure.
    Pressure,
    /// UV index.
    UvIndex,
    /// Cloud cover percentage.
    CloudCover,
    /// Sunshine duration for today.
    Sunshine,
    /// Precipitation probability for today.
    PrecipitationProbability,
    /// Precipitation amount sum for today.
    PrecipitationAmount,
    /// Current precipitation (rain, showers, snowfall).
    Precipitation,
}
