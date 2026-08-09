use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandDirection;

/// Direction for movement and focus commands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpDirection {
    /// Move or focus up.
    Up,
    /// Move or focus down.
    Down,
    /// Move or focus left.
    #[default]
    Left,
    /// Move or focus right.
    Right,
}

impl From<McpDirection> for HyprlandDirection {
    fn from(value: McpDirection) -> Self {
        match value {
            McpDirection::Up => HyprlandDirection::Up,
            McpDirection::Down => HyprlandDirection::Down,
            McpDirection::Left => HyprlandDirection::Left,
            McpDirection::Right => HyprlandDirection::Right,
        }
    }
}
