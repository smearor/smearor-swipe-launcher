use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandMonitorIdentifierKind;

/// The kind of monitor identifier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpMonitorIdentifierKind {
    /// The current monitor.
    #[default]
    Current,
    /// A monitor identified by a direction (left, right, up, down).
    Direction,
    /// A monitor identified by its numeric id.
    Id,
    /// A monitor identified by its name.
    Name,
    /// A monitor identified by a relative offset.
    Relative,
}

impl From<McpMonitorIdentifierKind> for HyprlandMonitorIdentifierKind {
    fn from(value: McpMonitorIdentifierKind) -> Self {
        match value {
            McpMonitorIdentifierKind::Current => HyprlandMonitorIdentifierKind::Current,
            McpMonitorIdentifierKind::Direction => HyprlandMonitorIdentifierKind::Direction,
            McpMonitorIdentifierKind::Id => HyprlandMonitorIdentifierKind::Id,
            McpMonitorIdentifierKind::Name => HyprlandMonitorIdentifierKind::Name,
            McpMonitorIdentifierKind::Relative => HyprlandMonitorIdentifierKind::Relative,
        }
    }
}
