use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::ColorScheme;
use crate::DateFormat;
use crate::FirstDayOfWeek;
use crate::GeoCoordinates;
use crate::MeasurementSystem;
use crate::TOPIC_STATUS;
use crate::TemperatureUnit;
use crate::TimeFormat;
use crate::WindSpeedUnit;

/// Complete personalization profile broadcast by the service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PersonalizationStatusMessage {
    /// Geographic coordinates of the user's location.
    pub coordinates: stabby::option::Option<GeoCoordinates>,
    /// IANA timezone identifier (e.g. "Europe/Berlin").
    pub timezone: stabby::option::Option<stabby::string::String>,
    /// System locale string (e.g. "de-DE", "en-US").
    pub locale: stabby::option::Option<stabby::string::String>,
    /// Preferred temperature unit.
    pub temperature_unit: TemperatureUnit,
    /// Preferred wind speed unit.
    pub wind_speed_unit: WindSpeedUnit,
    /// Preferred time format (12h/24h).
    pub time_format: TimeFormat,
    /// Preferred date format.
    pub date_format: DateFormat,
    /// First day of the week.
    pub first_day_of_week: FirstDayOfWeek,
    /// Preferred measurement system (metric/imperial).
    pub measurement_system: MeasurementSystem,
    /// Preferred color scheme (light/dark/system).
    pub color_scheme: ColorScheme,
    /// Whether the data was fetched successfully.
    pub success: bool,
    /// Error message if fetching failed.
    pub error_message: stabby::option::Option<stabby::string::String>,
}

impl PersonalizationStatusMessage {
    /// Creates a new personalization status message.
    pub fn new() -> Self {
        Self::default()
    }
}

impl TypedMessage for PersonalizationStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_personalization_model::PersonalizationStatusMessage");
}

impl MessageTopic for PersonalizationStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl SharedMessage for PersonalizationStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_STATUS
    }
}
