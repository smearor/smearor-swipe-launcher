use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandWorkspaceIdentifierKind;

/// The kind of workspace identifier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpWorkspaceIdentifierKind {
    /// The previously active workspace.
    #[default]
    Previous,
    /// The first empty workspace.
    Empty,
    /// A workspace identified by its numeric id.
    Id,
    /// A workspace identified by a relative offset.
    Relative,
    /// A workspace identified by a relative offset on the current monitor.
    RelativeMonitor,
    /// A workspace identified by a relative offset on the current monitor, including empty workspaces.
    RelativeMonitorIncludingEmpty,
    /// The next open workspace identified by a relative offset.
    RelativeOpen,
    /// A workspace identified by its name.
    Name,
    /// A special workspace (e.g. scratchpad).
    Special,
}

impl From<McpWorkspaceIdentifierKind> for HyprlandWorkspaceIdentifierKind {
    fn from(value: McpWorkspaceIdentifierKind) -> Self {
        match value {
            McpWorkspaceIdentifierKind::Previous => HyprlandWorkspaceIdentifierKind::Previous,
            McpWorkspaceIdentifierKind::Empty => HyprlandWorkspaceIdentifierKind::Empty,
            McpWorkspaceIdentifierKind::Id => HyprlandWorkspaceIdentifierKind::Id,
            McpWorkspaceIdentifierKind::Relative => HyprlandWorkspaceIdentifierKind::Relative,
            McpWorkspaceIdentifierKind::RelativeMonitor => HyprlandWorkspaceIdentifierKind::RelativeMonitor,
            McpWorkspaceIdentifierKind::RelativeMonitorIncludingEmpty => HyprlandWorkspaceIdentifierKind::RelativeMonitorIncludingEmpty,
            McpWorkspaceIdentifierKind::RelativeOpen => HyprlandWorkspaceIdentifierKind::RelativeOpen,
            McpWorkspaceIdentifierKind::Name => HyprlandWorkspaceIdentifierKind::Name,
            McpWorkspaceIdentifierKind::Special => HyprlandWorkspaceIdentifierKind::Special,
        }
    }
}
