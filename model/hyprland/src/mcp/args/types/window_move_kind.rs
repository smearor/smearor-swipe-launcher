use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandWindowMoveKind;

/// The kind of window move.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpWindowMoveKind {
    /// Move the window in a specific direction.
    #[default]
    Direction,
    /// Move the window to a specific monitor.
    Monitor,
}

impl From<McpWindowMoveKind> for HyprlandWindowMoveKind {
    fn from(value: McpWindowMoveKind) -> Self {
        match value {
            McpWindowMoveKind::Direction => HyprlandWindowMoveKind::Direction,
            McpWindowMoveKind::Monitor => HyprlandWindowMoveKind::Monitor,
        }
    }
}
