use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandPosition;

use crate::mcp::args::types::position_kind::McpPositionKind;

/// Position for move/resize commands, expressed as either a delta or an exact coordinate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct McpPosition {
    /// The kind of position.
    pub kind: McpPositionKind,
    /// The x coordinate.
    pub x: i16,
    /// The y coordinate.
    pub y: i16,
}

impl From<McpPosition> for HyprlandPosition {
    fn from(value: McpPosition) -> Self {
        HyprlandPosition {
            kind: value.kind.into(),
            x: value.x,
            y: value.y,
        }
    }
}
