use smearor_personalization_model::GeoCoordinates;

/// Internal command enum for the async update loop.
#[derive(Debug, Clone)]
pub(crate) enum PersonalizationCommand {
    /// Clear all runtime overrides and re-query system APIs.
    Refresh,
    /// Set a runtime location override.
    UpdateLocation(GeoCoordinates),
    /// Set a runtime locale override.
    UpdateLocale(String),
    /// Re-broadcast current status without clearing overrides.
    RequestStatus,
}
