use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandWorkspaceOptions;

/// Workspace layout options for the workspace-toggle command.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpWorkspaceOptions {
    /// Set all workspaces to pseudo-tiling mode.
    #[default]
    AllPseudo,
    /// Set all workspaces to floating mode.
    AllFloat,
}

impl From<McpWorkspaceOptions> for HyprlandWorkspaceOptions {
    fn from(value: McpWorkspaceOptions) -> Self {
        match value {
            McpWorkspaceOptions::AllPseudo => HyprlandWorkspaceOptions::AllPseudo,
            McpWorkspaceOptions::AllFloat => HyprlandWorkspaceOptions::AllFloat,
        }
    }
}
