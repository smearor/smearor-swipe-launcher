use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandCycleDirection;

/// Direction for cycling through windows or workspaces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpCycleDirection {
    /// Cycle to the next item.
    #[default]
    Next,
    /// Cycle to the previous item.
    Previous,
}

impl From<McpCycleDirection> for HyprlandCycleDirection {
    fn from(value: McpCycleDirection) -> Self {
        match value {
            McpCycleDirection::Next => HyprlandCycleDirection::Next,
            McpCycleDirection::Previous => HyprlandCycleDirection::Previous,
        }
    }
}
