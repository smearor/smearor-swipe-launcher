use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `get_current_time` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct GetCurrentTimeArgs {}
