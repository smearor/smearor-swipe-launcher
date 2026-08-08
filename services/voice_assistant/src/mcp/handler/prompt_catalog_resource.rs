use serde::Serialize;

use smearor_voice_assistant_model::PromptCatalogEntry;

/// JSON response payload for the `voice_assistant://prompt_catalog` resource.
///
/// Wraps the prompt catalog entries in a top-level `prompts` array for
/// structured JSON serialization.
#[derive(Serialize)]
pub struct PromptCatalogResourceResponse {
    /// List of all discovered prompt catalog entries.
    pub prompts: Vec<PromptCatalogEntry>,
}

impl PromptCatalogResourceResponse {
    /// Create a new response from an iterator of prompt catalog entries.
    pub fn new(prompts: Vec<PromptCatalogEntry>) -> Self {
        Self { prompts }
    }
}
