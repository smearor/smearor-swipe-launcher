use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandWindowSwitchDirection;

/// Direction for switching between windows in the stack.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpWindowSwitchDirection {
    /// Switch to the previous window in the stack.
    #[default]
    Back,
    /// Switch to the next window in the stack.
    Forward,
}

impl From<McpWindowSwitchDirection> for HyprlandWindowSwitchDirection {
    fn from(value: McpWindowSwitchDirection) -> Self {
        match value {
            McpWindowSwitchDirection::Back => HyprlandWindowSwitchDirection::Back,
            McpWindowSwitchDirection::Forward => HyprlandWindowSwitchDirection::Forward,
        }
    }
}
