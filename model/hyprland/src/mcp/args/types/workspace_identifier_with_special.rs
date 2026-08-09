use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandWorkspaceIdentifierWithSpecial;

use crate::mcp::args::types::workspace_identifier_kind::McpWorkspaceIdentifierKind;

/// Identifies a workspace, including special workspaces.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct McpWorkspaceIdentifierWithSpecial {
    /// The kind of identifier.
    pub kind: McpWorkspaceIdentifierKind,
    /// Numeric value for Id/Relative variants.
    pub id: i32,
    /// Name value for the Name variant.
    pub name: Option<String>,
    /// Optional name for the Special variant.
    pub special_name: Option<String>,
}

impl From<McpWorkspaceIdentifierWithSpecial> for HyprlandWorkspaceIdentifierWithSpecial {
    fn from(value: McpWorkspaceIdentifierWithSpecial) -> Self {
        HyprlandWorkspaceIdentifierWithSpecial {
            kind: value.kind.into(),
            id: value.id,
            name: value.name.map(stabby::string::String::from).into(),
            special_name: value.special_name.map(stabby::string::String::from).into(),
        }
    }
}
