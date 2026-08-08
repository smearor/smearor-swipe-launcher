use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `audio_set_volume` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct AudioSetVolumeArgs {
    /// Absolute volume level between 0.0 and 1.0
    pub volume: f32,
}
