use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandCorner;

/// Corner position for move-to-corner commands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpCorner {
    /// Bottom-left corner of the screen.
    #[default]
    BottomLeft,
    /// Bottom-right corner of the screen.
    BottomRight,
    /// Top-right corner of the screen.
    TopRight,
    /// Top-left corner of the screen.
    TopLeft,
}

impl From<McpCorner> for HyprlandCorner {
    fn from(value: McpCorner) -> Self {
        match value {
            McpCorner::BottomLeft => HyprlandCorner::BottomLeft,
            McpCorner::BottomRight => HyprlandCorner::BottomRight,
            McpCorner::TopRight => HyprlandCorner::TopRight,
            McpCorner::TopLeft => HyprlandCorner::TopLeft,
        }
    }
}
