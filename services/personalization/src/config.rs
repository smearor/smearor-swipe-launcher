use serde::Deserialize;

/// Configuration overrides for personalization data.
///
/// All fields are optional. When present, they override the auto-detected
/// system values. Runtime overrides (via command messages) take priority
/// over these config values.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct PersonalizationServiceConfig {
    /// Fixed latitude override.
    #[serde(default)]
    pub latitude: Option<f64>,
    /// Fixed longitude override.
    #[serde(default)]
    pub longitude: Option<f64>,
    /// Fixed location name override.
    #[serde(default)]
    pub location_name: Option<String>,
    /// Fixed timezone override (IANA identifier, e.g. "Europe/Berlin").
    #[serde(default)]
    pub timezone: Option<String>,
    /// Fixed locale override (e.g. "de-DE", "en-US").
    #[serde(default)]
    pub locale: Option<String>,
    /// Fixed temperature unit override.
    #[serde(default)]
    pub temperature_unit: Option<String>,
    /// Fixed wind speed unit override.
    #[serde(default)]
    pub wind_speed_unit: Option<String>,
    /// Fixed time format override.
    #[serde(default)]
    pub time_format: Option<String>,
    /// Fixed date format override.
    #[serde(default)]
    pub date_format: Option<String>,
    /// Fixed first day of week override.
    #[serde(default)]
    pub first_day_of_week: Option<String>,
    /// Fixed measurement system override.
    #[serde(default)]
    pub measurement_system: Option<String>,
    /// Fixed color scheme override.
    #[serde(default)]
    pub color_scheme: Option<String>,
    /// Whether to enable location detection via XDG Desktop Portal.
    #[serde(default = "default_enable_location")]
    pub enable_location: bool,
    /// Update interval in seconds for periodic system API re-queries.
    #[serde(default = "default_update_interval_seconds")]
    pub update_interval_seconds: u64,
    /// Interval in seconds for re-querying the XDG Desktop Portal location.
    /// Defaults to 1800 (30 minutes). Only relevant when `enable_location` is true.
    #[serde(default = "default_location_update_interval_seconds")]
    pub location_update_interval_seconds: u64,
    /// Threshold in degrees for significant location change.
    /// If the new coordinates differ from cached coordinates by less than this threshold,
    /// the location update (including reverse geocoding) is skipped.
    /// Defaults to 0.01 degrees (~1.1 km).
    #[serde(default = "default_location_change_threshold")]
    pub location_change_threshold: f64,
}

fn default_enable_location() -> bool {
    false
}

fn default_update_interval_seconds() -> u64 {
    300
}

fn default_location_update_interval_seconds() -> u64 {
    1800
}

fn default_location_change_threshold() -> f64 {
    0.01
}
