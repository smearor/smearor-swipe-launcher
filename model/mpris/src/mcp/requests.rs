use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `mpris_seek` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MprisSeekArgs {
    /// Seek offset in microseconds (positive or negative)
    pub offset: i64,
}

/// Arguments for the `mpris_set_position` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MprisSetPositionArgs {
    /// Absolute position in microseconds
    pub position: i64,
}
