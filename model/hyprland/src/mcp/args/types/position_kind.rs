use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandPositionKind;

/// The kind of position (delta or exact).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpPositionKind {
    /// Position relative to the current position (delta offset).
    #[default]
    Delta,
    /// Absolute position on the screen.
    Exact,
}

impl From<McpPositionKind> for HyprlandPositionKind {
    fn from(value: McpPositionKind) -> Self {
        match value {
            McpPositionKind::Delta => HyprlandPositionKind::Delta,
            McpPositionKind::Exact => HyprlandPositionKind::Exact,
        }
    }
}
