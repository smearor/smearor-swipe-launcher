use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use super::types::McpMonitorIdentifier;
use super::types::McpWorkspaceIdentifier;
use super::types::McpWorkspaceIdentifierWithSpecial;
use super::types::McpWorkspaceOptions;

/// Arguments for the `hyprland_workspace_move_current_to_monitor` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MoveCurrentWorkspaceToMonitorArgs {
    /// Monitor identifier (current, direction, id, name, or relative)
    pub monitor_identifier: McpMonitorIdentifier,
}

/// Arguments for the `hyprland_workspace_move_focused_window` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MoveFocusedWindowToWorkspaceArgs {
    /// Workspace identifier (previous, empty, id, relative, name, etc.)
    pub identifier: McpWorkspaceIdentifier,
}

/// Arguments for the `hyprland_workspace_move_focused_window_silent` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MoveFocusedWindowToWorkspaceSilentArgs {
    /// Workspace identifier (previous, empty, id, relative, name, etc.)
    pub identifier: McpWorkspaceIdentifier,
}

/// Arguments for the `hyprland_workspace_move_to_workspace_silent` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MoveToWorkspaceSilentArgs {
    /// Workspace identifier (previous, empty, id, relative, name, special, etc.)
    pub identifier: McpWorkspaceIdentifierWithSpecial,
    /// Optional window identifier; if omitted, the active window is moved
    pub window_identifier: Option<super::types::McpWindowIdentifier>,
}

/// Arguments for the `hyprland_workspace_rename` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct RenameWorkspaceArgs {
    /// The workspace ID to rename
    pub workspace_id: i32,
    /// The new name for the workspace, or None to clear the name
    pub new_name: Option<String>,
}

/// Arguments for the `hyprland_workspace_swap_active` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SwapActiveWorkspacesArgs {
    /// First monitor identifier
    pub monitor_a: McpMonitorIdentifier,
    /// Second monitor identifier
    pub monitor_b: McpMonitorIdentifier,
}

/// Arguments for the `hyprland_workspace_toggle_special` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SpecialWorkspaceArgs {
    /// Optional name of the special workspace to toggle
    pub workspace_name: Option<String>,
}

/// Arguments for the `hyprland_workspace_option` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceOptionArgs {
    /// The workspace option to toggle (all pseudo or all float)
    pub option: McpWorkspaceOptions,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_workspace_args_roundtrip() {
        let original = RenameWorkspaceArgs {
            workspace_id: 3,
            new_name: Some("dev".to_string()),
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: RenameWorkspaceArgs = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.workspace_id, 3);
        assert_eq!(parsed.new_name, Some("dev".to_string()));
    }

    #[test]
    fn rename_workspace_args_with_null_name() {
        let json = r#"{"workspace_id":5,"new_name":null}"#;
        let parsed: RenameWorkspaceArgs = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.workspace_id, 5);
        assert_eq!(parsed.new_name, None);
    }

    #[test]
    fn special_workspace_args_default() {
        let args: SpecialWorkspaceArgs = serde_json::from_str("{}").unwrap_or_default();
        assert_eq!(args.workspace_name, None);
    }

    #[test]
    fn swap_active_workspaces_args_default() {
        let args: SwapActiveWorkspacesArgs = serde_json::from_str("not json").unwrap_or_default();
        assert_eq!(args.monitor_a, McpMonitorIdentifier::default());
        assert_eq!(args.monitor_b, McpMonitorIdentifier::default());
    }
}
