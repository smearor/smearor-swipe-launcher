use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandWorkspaceIdentifier;

/// Identifies a workspace without special workspace support.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpWorkspaceIdentifier {
    /// The previously active workspace.
    #[default]
    Previous,
    /// The first empty workspace.
    Empty,
    /// A workspace identified by its numeric id.
    Id(i32),
    /// A workspace identified by a relative offset.
    Relative(i32),
    /// A workspace identified by a relative offset on the current monitor.
    RelativeMonitor(i32),
    /// A workspace identified by a relative offset on the current monitor, including empty workspaces.
    RelativeMonitorIncludingEmpty(i32),
    /// The next open workspace identified by a relative offset.
    RelativeOpen(i32),
    /// A workspace identified by its name.
    Name(String),
}

impl From<McpWorkspaceIdentifier> for HyprlandWorkspaceIdentifier {
    fn from(value: McpWorkspaceIdentifier) -> Self {
        match value {
            McpWorkspaceIdentifier::Previous => HyprlandWorkspaceIdentifier::Previous(),
            McpWorkspaceIdentifier::Empty => HyprlandWorkspaceIdentifier::Empty(),
            McpWorkspaceIdentifier::Id(id) => HyprlandWorkspaceIdentifier::Id(id),
            McpWorkspaceIdentifier::Relative(offset) => HyprlandWorkspaceIdentifier::Relative(offset),
            McpWorkspaceIdentifier::RelativeMonitor(offset) => HyprlandWorkspaceIdentifier::RelativeMonitor(offset),
            McpWorkspaceIdentifier::RelativeMonitorIncludingEmpty(offset) => HyprlandWorkspaceIdentifier::RelativeMonitorIncludingEmpty(offset),
            McpWorkspaceIdentifier::RelativeOpen(offset) => HyprlandWorkspaceIdentifier::RelativeOpen(offset),
            McpWorkspaceIdentifier::Name(name) => HyprlandWorkspaceIdentifier::Name(name.into()),
        }
    }
}
