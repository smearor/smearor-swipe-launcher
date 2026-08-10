use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandWorkspaceIdentifier;

use crate::mcp::args::types::workspace_identifier_kind::McpWorkspaceIdentifierKind;

/// Identifies a workspace without special workspace support.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct McpWorkspaceIdentifier {
    /// The kind of identifier.
    pub kind: McpWorkspaceIdentifierKind,
    /// Numeric value for Id/Relative variants.
    pub id: i32,
    /// Name value for the Name variant.
    pub name: Option<String>,
}

impl From<McpWorkspaceIdentifier> for HyprlandWorkspaceIdentifier {
    fn from(value: McpWorkspaceIdentifier) -> Self {
        match value.kind {
            McpWorkspaceIdentifierKind::Previous => HyprlandWorkspaceIdentifier::Previous(),
            McpWorkspaceIdentifierKind::Empty => HyprlandWorkspaceIdentifier::Empty(),
            McpWorkspaceIdentifierKind::Id => HyprlandWorkspaceIdentifier::Id(value.id),
            McpWorkspaceIdentifierKind::Relative => HyprlandWorkspaceIdentifier::Relative(value.id),
            McpWorkspaceIdentifierKind::RelativeMonitor => HyprlandWorkspaceIdentifier::RelativeMonitor(value.id),
            McpWorkspaceIdentifierKind::RelativeMonitorIncludingEmpty => HyprlandWorkspaceIdentifier::RelativeMonitorIncludingEmpty(value.id),
            McpWorkspaceIdentifierKind::RelativeOpen => HyprlandWorkspaceIdentifier::RelativeOpen(value.id),
            McpWorkspaceIdentifierKind::Name => HyprlandWorkspaceIdentifier::Name(value.name.unwrap_or_default().into()),
            McpWorkspaceIdentifierKind::Special => HyprlandWorkspaceIdentifier::Previous(),
        }
    }
}
