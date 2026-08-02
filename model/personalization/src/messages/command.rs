use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::GeoCoordinates;
use crate::TOPIC_COMMAND;

/// Actions the personalization service can perform on request.
#[repr(C, u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PersonalizationCommandAction {
    /// Force an immediate refresh of all personalization data from system APIs.
    /// Clears all runtime overrides and re-queries system APIs.
    Refresh,
    /// Update the user's location at runtime.
    /// The service stores the new coordinates and broadcasts an updated status.
    /// This overrides any config or auto-detected value until a `Refresh` is triggered
    /// or the service is restarted.
    UpdateLocation(GeoCoordinates),
    /// Update the user's locale at runtime.
    /// The service stores the new locale, re-derives unit/format preferences,
    /// and broadcasts an updated status.
    /// This overrides any config or auto-detected value until a `Refresh` is triggered
    /// or the service is restarted.
    UpdateLocale(stabby::string::String),
    /// Request an immediate status re-broadcast without clearing runtime overrides.
    /// Used by widgets that are lazily loaded after the initial status broadcast.
    RequestStatus,
}

/// Command message sent by consumers to the personalization service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersonalizationCommandMessage {
    /// The action to execute.
    pub action: PersonalizationCommandAction,
}

impl PersonalizationCommandMessage {
    /// Creates a new command message with the given action.
    pub fn new(action: PersonalizationCommandAction) -> Self {
        Self { action }
    }

    /// Creates a refresh command message.
    pub fn refresh() -> Self {
        Self::new(PersonalizationCommandAction::Refresh)
    }

    /// Creates an update-location command message.
    pub fn update_location(coordinates: GeoCoordinates) -> Self {
        Self::new(PersonalizationCommandAction::UpdateLocation(coordinates))
    }

    /// Creates an update-locale command message.
    pub fn update_locale(locale: &str) -> Self {
        Self::new(PersonalizationCommandAction::UpdateLocale(locale.into()))
    }

    /// Creates a request-status command message.
    pub fn request_status() -> Self {
        Self::new(PersonalizationCommandAction::RequestStatus)
    }
}

impl Default for PersonalizationCommandMessage {
    fn default() -> Self {
        Self::refresh()
    }
}

impl TypedMessage for PersonalizationCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_personalization_model::PersonalizationCommandMessage");
}

impl MessageTopic for PersonalizationCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl SharedMessage for PersonalizationCommandMessage {
    fn topic(&self) -> &'static str {
        TOPIC_COMMAND
    }
}
