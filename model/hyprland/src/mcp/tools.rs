use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the hyprland service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HyprlandMcpTools {
    // Window dispatch tools
    /// Center the active window on screen.
    WindowCenter,
    /// Change active window in a group.
    WindowChangeGroupActive,
    /// Change the split ratio of the active window.
    WindowChangeSplitRatio,
    /// Close a specific window.
    WindowClose,
    /// Cycle focus to the next or previous window.
    WindowCycle,
    /// Execute a command via Hyprland dispatch.
    WindowExec,
    /// Focus the current or last focused window.
    WindowFocusCurrentOrLast,
    /// Focus the master window.
    WindowFocusMaster,
    /// Focus a specific monitor.
    WindowFocusMonitor,
    /// Focus the urgent or last focused window.
    WindowFocusUrgentOrLast,
    /// Focus a specific window.
    WindowFocusWindow,
    /// Kill the active window.
    WindowKillActive,
    /// Move the active window by pixel delta or to exact position.
    WindowMoveActive,
    /// Move the cursor to specified coordinates.
    WindowMoveCursor,
    /// Move the cursor to a corner of the active window.
    WindowMoveCursorToCorner,
    /// Move focus in a direction.
    WindowMoveFocus,
    /// Move the active window into a group.
    WindowMoveIntoGroup,
    /// Move a window by direction or to a monitor.
    WindowMoveWindow,
    /// Move a specific window by pixel delta.
    WindowMoveWindowPixel,
    /// Resize the active window.
    WindowResizeActive,
    /// Resize a specific window by pixel delta.
    WindowResizeWindowPixel,
    /// Swap the active window with next or previous.
    WindowSwap,
    /// Swap the active window with the master.
    WindowSwapWithMaster,

    // Workspace dispatch tools
    /// Move the active window to a workspace.
    MoveWindowToWorkspace,
    /// Move the current workspace to a specific monitor.
    WorkspaceMoveCurrentToMonitor,
    /// Move the focused window to a workspace.
    WorkspaceMoveFocusedWindow,
    /// Move the focused window to a workspace silently.
    WorkspaceMoveFocusedWindowSilent,
    /// Move the active window to a workspace silently.
    WorkspaceMoveToWorkspaceSilent,
    /// Rename a workspace.
    WorkspaceRename,
    /// Swap active workspaces between two monitors.
    WorkspaceSwapActive,
    /// Toggle a special workspace.
    WorkspaceToggleSpecial,
    /// Set a workspace option.
    WorkspaceOption,

    // Toggle dispatch tools
    /// Toggle floating mode for the active window.
    ToggleFloating,
    /// Toggle fullscreen for the active window.
    ToggleFullscreen,
    /// Toggle DPMS (display power management).
    ToggleDpms,
    /// Toggle fake fullscreen.
    ToggleFakeFullscreen,
    /// Toggle window group.
    ToggleGroup,
    /// Toggle opaque.
    ToggleOpaque,
    /// Toggle pin.
    TogglePin,
    /// Toggle pseudo tiling.
    TogglePseudo,
    /// Toggle split.
    ToggleSplit,

    // System dispatch tools
    /// Add a master to the layout.
    SystemAddMaster,
    /// Bring the active window to top.
    SystemBringActiveToTop,
    /// Execute a custom Hyprland dispatch.
    SystemCustom,
    /// Exit Hyprland.
    SystemExit,
    /// Force renderer reload.
    SystemForceRendererReload,
    /// Execute a global keybinding.
    SystemGlobal,
    /// Lock, unlock, or toggle group locks.
    SystemLockGroups,
    /// Move the active window out of its group.
    SystemMoveOutOfGroup,
    /// Set window orientation.
    SystemOrientation,
    /// Pass a key event to a window.
    SystemPass,
    /// Remove a master from the layout.
    SystemRemoveMaster,
    /// Set the cursor theme and size.
    SystemSetCursor,

    // Control command tools
    /// Enter kill mode.
    CtlKill,
    /// Send a Hyprland notification.
    CtlNotify,
    /// Create a virtual output.
    CtlOutputCreate,
    /// Remove a virtual output.
    CtlOutputRemove,
    /// Load a Hyprland plugin.
    CtlPluginLoad,
    /// Unload a Hyprland plugin.
    CtlPluginUnload,
    /// Reload Hyprland configuration.
    CtlReload,
    /// Set cursor (ctl variant).
    CtlSetCursor,
    /// Set an error status.
    CtlSetError,
    /// Set a window property.
    CtlSetProp,
    /// Switch the XKB keyboard layout.
    CtlSwitchXkbLayout,

    // Compositor workspace tools
    /// Switch to a workspace by ID.
    SwitchWorkspace,
    /// Create a new workspace.
    CompositorCreateWorkspace,
    /// Switch workspace (compositor-level).
    CompositorSwitchWorkspace,

    // State refresh tools
    /// Refresh the Hyprland state and workspace snapshot from the compositor.
    RefreshState,
}

impl AsRef<str> for HyprlandMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::WindowCenter => "hyprland_window_center",
            Self::WindowChangeGroupActive => "hyprland_window_change_group_active",
            Self::WindowChangeSplitRatio => "hyprland_window_change_split_ratio",
            Self::WindowClose => "hyprland_window_close",
            Self::WindowCycle => "hyprland_window_cycle",
            Self::WindowExec => "hyprland_window_exec",
            Self::WindowFocusCurrentOrLast => "hyprland_window_focus_current_or_last",
            Self::WindowFocusMaster => "hyprland_window_focus_master",
            Self::WindowFocusMonitor => "hyprland_window_focus_monitor",
            Self::WindowFocusUrgentOrLast => "hyprland_window_focus_urgent_or_last",
            Self::WindowFocusWindow => "hyprland_window_focus_window",
            Self::WindowKillActive => "hyprland_window_kill_active",
            Self::WindowMoveActive => "hyprland_window_move_active",
            Self::WindowMoveCursor => "hyprland_window_move_cursor",
            Self::WindowMoveCursorToCorner => "hyprland_window_move_cursor_to_corner",
            Self::WindowMoveFocus => "hyprland_window_move_focus",
            Self::WindowMoveIntoGroup => "hyprland_window_move_into_group",
            Self::WindowMoveWindow => "hyprland_window_move_window",
            Self::WindowMoveWindowPixel => "hyprland_window_move_window_pixel",
            Self::WindowResizeActive => "hyprland_window_resize_active",
            Self::WindowResizeWindowPixel => "hyprland_window_resize_window_pixel",
            Self::WindowSwap => "hyprland_window_swap",
            Self::WindowSwapWithMaster => "hyprland_window_swap_with_master",
            Self::MoveWindowToWorkspace => "hyprland_move_window",
            Self::WorkspaceMoveCurrentToMonitor => "hyprland_workspace_move_current_to_monitor",
            Self::WorkspaceMoveFocusedWindow => "hyprland_workspace_move_focused_window",
            Self::WorkspaceMoveFocusedWindowSilent => "hyprland_workspace_move_focused_window_silent",
            Self::WorkspaceMoveToWorkspaceSilent => "hyprland_workspace_move_to_workspace_silent",
            Self::WorkspaceRename => "hyprland_workspace_rename",
            Self::WorkspaceSwapActive => "hyprland_workspace_swap_active",
            Self::WorkspaceToggleSpecial => "hyprland_workspace_toggle_special",
            Self::WorkspaceOption => "hyprland_workspace_option",
            Self::ToggleFloating => "hyprland_toggle_floating",
            Self::ToggleFullscreen => "hyprland_toggle_fullscreen",
            Self::ToggleDpms => "hyprland_toggle_dpms",
            Self::ToggleFakeFullscreen => "hyprland_toggle_fake_fullscreen",
            Self::ToggleGroup => "hyprland_toggle_group",
            Self::ToggleOpaque => "hyprland_toggle_opaque",
            Self::TogglePin => "hyprland_toggle_pin",
            Self::TogglePseudo => "hyprland_toggle_pseudo",
            Self::ToggleSplit => "hyprland_toggle_split",
            Self::SystemAddMaster => "hyprland_system_add_master",
            Self::SystemBringActiveToTop => "hyprland_system_bring_active_to_top",
            Self::SystemCustom => "hyprland_system_custom",
            Self::SystemExit => "hyprland_system_exit",
            Self::SystemForceRendererReload => "hyprland_system_force_renderer_reload",
            Self::SystemGlobal => "hyprland_system_global",
            Self::SystemLockGroups => "hyprland_system_lock_groups",
            Self::SystemMoveOutOfGroup => "hyprland_system_move_out_of_group",
            Self::SystemOrientation => "hyprland_system_orientation",
            Self::SystemPass => "hyprland_system_pass",
            Self::SystemRemoveMaster => "hyprland_system_remove_master",
            Self::SystemSetCursor => "hyprland_system_set_cursor",
            Self::CtlKill => "hyprland_ctl_kill",
            Self::CtlNotify => "hyprland_ctl_notify",
            Self::CtlOutputCreate => "hyprland_ctl_output_create",
            Self::CtlOutputRemove => "hyprland_ctl_output_remove",
            Self::CtlPluginLoad => "hyprland_ctl_plugin_load",
            Self::CtlPluginUnload => "hyprland_ctl_plugin_unload",
            Self::CtlReload => "hyprland_ctl_reload",
            Self::CtlSetCursor => "hyprland_ctl_set_cursor",
            Self::CtlSetError => "hyprland_ctl_set_error",
            Self::CtlSetProp => "hyprland_ctl_set_prop",
            Self::CtlSwitchXkbLayout => "hyprland_ctl_switch_xkb_layout",
            Self::SwitchWorkspace => "hyprland_switch_workspace",
            Self::CompositorCreateWorkspace => "hyprland_compositor_create_workspace",
            Self::CompositorSwitchWorkspace => "hyprland_compositor_switch_workspace",
            Self::RefreshState => "hyprland_refresh_state",
        }
    }
}

impl FromStr for HyprlandMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "hyprland_window_center" => Ok(Self::WindowCenter),
            "hyprland_window_change_group_active" => Ok(Self::WindowChangeGroupActive),
            "hyprland_window_change_split_ratio" => Ok(Self::WindowChangeSplitRatio),
            "hyprland_window_close" => Ok(Self::WindowClose),
            "hyprland_window_cycle" => Ok(Self::WindowCycle),
            "hyprland_window_exec" => Ok(Self::WindowExec),
            "hyprland_window_focus_current_or_last" => Ok(Self::WindowFocusCurrentOrLast),
            "hyprland_window_focus_master" => Ok(Self::WindowFocusMaster),
            "hyprland_window_focus_monitor" => Ok(Self::WindowFocusMonitor),
            "hyprland_window_focus_urgent_or_last" => Ok(Self::WindowFocusUrgentOrLast),
            "hyprland_window_focus_window" => Ok(Self::WindowFocusWindow),
            "hyprland_window_kill_active" => Ok(Self::WindowKillActive),
            "hyprland_window_move_active" => Ok(Self::WindowMoveActive),
            "hyprland_window_move_cursor" => Ok(Self::WindowMoveCursor),
            "hyprland_window_move_cursor_to_corner" => Ok(Self::WindowMoveCursorToCorner),
            "hyprland_window_move_focus" => Ok(Self::WindowMoveFocus),
            "hyprland_window_move_into_group" => Ok(Self::WindowMoveIntoGroup),
            "hyprland_window_move_window" => Ok(Self::WindowMoveWindow),
            "hyprland_window_move_window_pixel" => Ok(Self::WindowMoveWindowPixel),
            "hyprland_window_resize_active" => Ok(Self::WindowResizeActive),
            "hyprland_window_resize_window_pixel" => Ok(Self::WindowResizeWindowPixel),
            "hyprland_window_swap" => Ok(Self::WindowSwap),
            "hyprland_window_swap_with_master" => Ok(Self::WindowSwapWithMaster),
            "hyprland_move_window" => Ok(Self::MoveWindowToWorkspace),
            "hyprland_workspace_move_current_to_monitor" => Ok(Self::WorkspaceMoveCurrentToMonitor),
            "hyprland_workspace_move_focused_window" => Ok(Self::WorkspaceMoveFocusedWindow),
            "hyprland_workspace_move_focused_window_silent" => Ok(Self::WorkspaceMoveFocusedWindowSilent),
            "hyprland_workspace_move_to_workspace_silent" => Ok(Self::WorkspaceMoveToWorkspaceSilent),
            "hyprland_workspace_rename" => Ok(Self::WorkspaceRename),
            "hyprland_workspace_swap_active" => Ok(Self::WorkspaceSwapActive),
            "hyprland_workspace_toggle_special" => Ok(Self::WorkspaceToggleSpecial),
            "hyprland_workspace_option" => Ok(Self::WorkspaceOption),
            "hyprland_toggle_floating" => Ok(Self::ToggleFloating),
            "hyprland_toggle_fullscreen" => Ok(Self::ToggleFullscreen),
            "hyprland_toggle_dpms" => Ok(Self::ToggleDpms),
            "hyprland_toggle_fake_fullscreen" => Ok(Self::ToggleFakeFullscreen),
            "hyprland_toggle_group" => Ok(Self::ToggleGroup),
            "hyprland_toggle_opaque" => Ok(Self::ToggleOpaque),
            "hyprland_toggle_pin" => Ok(Self::TogglePin),
            "hyprland_toggle_pseudo" => Ok(Self::TogglePseudo),
            "hyprland_toggle_split" => Ok(Self::ToggleSplit),
            "hyprland_system_add_master" => Ok(Self::SystemAddMaster),
            "hyprland_system_bring_active_to_top" => Ok(Self::SystemBringActiveToTop),
            "hyprland_system_custom" => Ok(Self::SystemCustom),
            "hyprland_system_exit" => Ok(Self::SystemExit),
            "hyprland_system_force_renderer_reload" => Ok(Self::SystemForceRendererReload),
            "hyprland_system_global" => Ok(Self::SystemGlobal),
            "hyprland_system_lock_groups" => Ok(Self::SystemLockGroups),
            "hyprland_system_move_out_of_group" => Ok(Self::SystemMoveOutOfGroup),
            "hyprland_system_orientation" => Ok(Self::SystemOrientation),
            "hyprland_system_pass" => Ok(Self::SystemPass),
            "hyprland_system_remove_master" => Ok(Self::SystemRemoveMaster),
            "hyprland_system_set_cursor" => Ok(Self::SystemSetCursor),
            "hyprland_ctl_kill" => Ok(Self::CtlKill),
            "hyprland_ctl_notify" => Ok(Self::CtlNotify),
            "hyprland_ctl_output_create" => Ok(Self::CtlOutputCreate),
            "hyprland_ctl_output_remove" => Ok(Self::CtlOutputRemove),
            "hyprland_ctl_plugin_load" => Ok(Self::CtlPluginLoad),
            "hyprland_ctl_plugin_unload" => Ok(Self::CtlPluginUnload),
            "hyprland_ctl_reload" => Ok(Self::CtlReload),
            "hyprland_ctl_set_cursor" => Ok(Self::CtlSetCursor),
            "hyprland_ctl_set_error" => Ok(Self::CtlSetError),
            "hyprland_ctl_set_prop" => Ok(Self::CtlSetProp),
            "hyprland_ctl_switch_xkb_layout" => Ok(Self::CtlSwitchXkbLayout),
            "hyprland_switch_workspace" => Ok(Self::SwitchWorkspace),
            "hyprland_compositor_create_workspace" => Ok(Self::CompositorCreateWorkspace),
            "hyprland_compositor_switch_workspace" => Ok(Self::CompositorSwitchWorkspace),
            "hyprland_refresh_state" => Ok(Self::RefreshState),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for HyprlandMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tool_names_roundtrip() {
        let all_tools = [
            HyprlandMcpTools::WindowCenter,
            HyprlandMcpTools::WindowChangeGroupActive,
            HyprlandMcpTools::WindowChangeSplitRatio,
            HyprlandMcpTools::WindowClose,
            HyprlandMcpTools::WindowCycle,
            HyprlandMcpTools::WindowExec,
            HyprlandMcpTools::WindowFocusCurrentOrLast,
            HyprlandMcpTools::WindowFocusMaster,
            HyprlandMcpTools::WindowFocusMonitor,
            HyprlandMcpTools::WindowFocusUrgentOrLast,
            HyprlandMcpTools::WindowFocusWindow,
            HyprlandMcpTools::WindowKillActive,
            HyprlandMcpTools::WindowMoveActive,
            HyprlandMcpTools::WindowMoveCursor,
            HyprlandMcpTools::WindowMoveCursorToCorner,
            HyprlandMcpTools::WindowMoveFocus,
            HyprlandMcpTools::WindowMoveIntoGroup,
            HyprlandMcpTools::WindowMoveWindow,
            HyprlandMcpTools::WindowMoveWindowPixel,
            HyprlandMcpTools::WindowResizeActive,
            HyprlandMcpTools::WindowResizeWindowPixel,
            HyprlandMcpTools::WindowSwap,
            HyprlandMcpTools::WindowSwapWithMaster,
            HyprlandMcpTools::MoveWindowToWorkspace,
            HyprlandMcpTools::WorkspaceMoveCurrentToMonitor,
            HyprlandMcpTools::WorkspaceMoveFocusedWindow,
            HyprlandMcpTools::WorkspaceMoveFocusedWindowSilent,
            HyprlandMcpTools::WorkspaceMoveToWorkspaceSilent,
            HyprlandMcpTools::WorkspaceRename,
            HyprlandMcpTools::WorkspaceSwapActive,
            HyprlandMcpTools::WorkspaceToggleSpecial,
            HyprlandMcpTools::WorkspaceOption,
            HyprlandMcpTools::ToggleFloating,
            HyprlandMcpTools::ToggleFullscreen,
            HyprlandMcpTools::ToggleDpms,
            HyprlandMcpTools::ToggleFakeFullscreen,
            HyprlandMcpTools::ToggleGroup,
            HyprlandMcpTools::ToggleOpaque,
            HyprlandMcpTools::TogglePin,
            HyprlandMcpTools::TogglePseudo,
            HyprlandMcpTools::ToggleSplit,
            HyprlandMcpTools::SystemAddMaster,
            HyprlandMcpTools::SystemBringActiveToTop,
            HyprlandMcpTools::SystemCustom,
            HyprlandMcpTools::SystemExit,
            HyprlandMcpTools::SystemForceRendererReload,
            HyprlandMcpTools::SystemGlobal,
            HyprlandMcpTools::SystemLockGroups,
            HyprlandMcpTools::SystemMoveOutOfGroup,
            HyprlandMcpTools::SystemOrientation,
            HyprlandMcpTools::SystemPass,
            HyprlandMcpTools::SystemRemoveMaster,
            HyprlandMcpTools::SystemSetCursor,
            HyprlandMcpTools::CtlKill,
            HyprlandMcpTools::CtlNotify,
            HyprlandMcpTools::CtlOutputCreate,
            HyprlandMcpTools::CtlOutputRemove,
            HyprlandMcpTools::CtlPluginLoad,
            HyprlandMcpTools::CtlPluginUnload,
            HyprlandMcpTools::CtlReload,
            HyprlandMcpTools::CtlSetCursor,
            HyprlandMcpTools::CtlSetError,
            HyprlandMcpTools::CtlSetProp,
            HyprlandMcpTools::CtlSwitchXkbLayout,
            HyprlandMcpTools::SwitchWorkspace,
            HyprlandMcpTools::CompositorCreateWorkspace,
            HyprlandMcpTools::CompositorSwitchWorkspace,
            HyprlandMcpTools::RefreshState,
        ];

        for tool in &all_tools {
            let name = tool.as_ref();
            let parsed = HyprlandMcpTools::from_str(name).unwrap_or_else(|_| panic!("failed to parse tool name: {name}"));
            assert_eq!(*tool, parsed, "tool roundtrip mismatch for {name}");
        }

        assert_eq!(all_tools.len(), 68, "expected 68 tool variants");
    }

    #[test]
    fn unknown_tool_name_returns_error() {
        assert!(HyprlandMcpTools::from_str("hyprland_nonexistent").is_err());
        assert!(HyprlandMcpTools::from_str("").is_err());
        assert!(HyprlandMcpTools::from_str("hyprland_window").is_err());
    }

    #[test]
    fn display_matches_as_ref() {
        let tool = HyprlandMcpTools::WindowCenter;
        assert_eq!(format!("{tool}"), tool.as_ref());
    }
}
