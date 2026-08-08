mod mcp;
mod messages;
mod topics;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use mcp::prompts::PersonalizationMcpPrompts;
pub use mcp::resources::PersonalizationMcpResources;
pub use mcp::tools::PersonalizationMcpTools;
pub use messages::color_scheme::ColorScheme;
pub use messages::command::PersonalizationCommandAction;
pub use messages::command::PersonalizationCommandMessage;
pub use messages::coordinates::GeoCoordinates;
pub use messages::date_format::DateFormat;
pub use messages::first_day_of_week::FirstDayOfWeek;
pub use messages::measurement_system::MeasurementSystem;
pub use messages::status::PersonalizationStatusMessage;
pub use messages::temperature_unit::TemperatureUnit;
pub use messages::time_format::TimeFormat;
pub use messages::wind_speed_unit::WindSpeedUnit;
pub use topics::TOPIC_COMMAND;
pub use topics::TOPIC_STATUS;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(PersonalizationCommandMessageConverter, PersonalizationCommandMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(PersonalizationStatusMessageConverter, PersonalizationStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

/// Register all JSON converter implementations for personalization messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    PersonalizationCommandMessageConverter::register_in_host(context);
    PersonalizationStatusMessageConverter::register_in_host(context);
}
