use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandWindowMove;

use crate::mcp::args::types::direction::McpDirection;
use crate::mcp::args::types::monitor_identifier::McpMonitorIdentifier;
use crate::mcp::args::types::window_move_kind::McpWindowMoveKind;

/// Parameters for moving a window, either by direction or to a specific monitor.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct McpWindowMove {
    /// The kind of move.
    pub kind: McpWindowMoveKind,
    /// Direction value for the Direction variant.
    pub direction: McpDirection,
    /// Monitor identifier for the Monitor variant.
    pub monitor: McpMonitorIdentifier,
}

impl From<McpWindowMove> for HyprlandWindowMove {
    fn from(value: McpWindowMove) -> Self {
        HyprlandWindowMove {
            kind: value.kind.into(),
            direction: value.direction.into(),
            monitor: value.monitor.into(),
        }
    }
}
