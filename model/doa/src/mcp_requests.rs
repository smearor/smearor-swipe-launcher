use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `doa_get_direction` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct DoaGetDirectionArgs {}

/// Arguments for the `doa_set_poll_interval` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct DoaSetPollIntervalArgs {
    /// Polling interval in milliseconds (min: 50, default: 150)
    pub interval_ms: u64,
}

/// Arguments for the `doa_reconnect` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct DoaReconnectArgs {}
