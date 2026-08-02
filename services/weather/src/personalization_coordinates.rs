/// Personalization override data for coordinates.
///
/// Stores the latitude and longitude received from the personalization service.
/// When available, these values override the static config coordinates for
/// weather data fetching.
#[derive(Clone, Debug, Default)]
pub struct PersonalizationCoordinates {
    /// Latitude in decimal degrees from personalization service.
    pub latitude: Option<f64>,
    /// Longitude in decimal degrees from personalization service.
    pub longitude: Option<f64>,
}
