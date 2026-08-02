use serde::Deserialize;
use serde::Serialize;

/// Geographic coordinates of the user's current location.
#[repr(C)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GeoCoordinates {
    /// Latitude in decimal degrees.
    pub latitude: f64,
    /// Longitude in decimal degrees.
    pub longitude: f64,
    /// Human-readable location name (reverse-geocoded or configured).
    pub location_name: stabby::option::Option<stabby::string::String>,
}
