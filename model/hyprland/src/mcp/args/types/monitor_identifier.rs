use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandMonitorIdentifier;

use crate::mcp::args::types::direction::McpDirection;
use crate::mcp::args::types::monitor_identifier_kind::McpMonitorIdentifierKind;

/// Identifies a monitor by direction, id, name, current, or relative offset.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct McpMonitorIdentifier {
    /// The kind of identifier.
    pub kind: McpMonitorIdentifierKind,
    /// Direction value for the Direction variant.
    pub direction: McpDirection,
    /// Numeric value for the Id variant.
    pub id: i32,
    /// Name value for the Name variant.
    pub name: Option<String>,
    /// Relative offset for the Relative variant.
    pub relative: i32,
}

impl From<McpMonitorIdentifier> for HyprlandMonitorIdentifier {
    fn from(value: McpMonitorIdentifier) -> Self {
        HyprlandMonitorIdentifier {
            kind: value.kind.into(),
            direction: value.direction.into(),
            id: value.id,
            name: value.name.map(stabby::string::String::from).into(),
            relative: value.relative,
        }
    }
}
