use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::TOPIC_COMMAND;

/// Actions that can be sent from the widget to the weather service.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub enum WeatherCommandAction {
    /// Force an immediate refresh of weather data.
    #[default]
    Refresh,
}

/// Command message sent from the weather widget to the weather service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct WeatherCommandMessage {
    /// The action to execute.
    pub action: WeatherCommandAction,
}

impl WeatherCommandMessage {
    /// Creates a new command message with the given action.
    pub fn new(action: WeatherCommandAction) -> Self {
        Self { action }
    }

    /// Creates a refresh command message.
    pub fn refresh() -> Self {
        Self::new(WeatherCommandAction::Refresh)
    }
}

impl TypedMessage for WeatherCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_weather_model::WeatherCommandMessage");
}

impl MessageTopic for WeatherCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl SharedMessage for WeatherCommandMessage {
    fn topic(&self) -> &'static str {
        TOPIC_COMMAND
    }
}
