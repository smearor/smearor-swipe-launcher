use smearor_personalization_model::PersonalizationStatusMessage;

/// Latest personalization state shared between the update loop and MCP handlers.
#[derive(Clone, Default)]
pub struct LatestPersonalizationState {
    /// Last broadcasted personalization status.
    pub status: PersonalizationStatusMessage,
}
