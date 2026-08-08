use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `system_power_action` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SystemPowerActionArgs {
    /// The power action to execute
    pub action: String,
}

/// Arguments for the `system_schedule_power_action` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SystemSchedulePowerActionArgs {
    /// The power action to schedule
    pub action: String,
    /// Delay in minutes before the action executes
    pub delay_minutes: u32,
}
