use crate::service::HyprlandService;
use schemars::schema_for;
use smearor_hyprland_model::AddMasterArgs;
use smearor_hyprland_model::BringActiveToTopArgs;
use smearor_hyprland_model::CenterWindowArgs;
use smearor_hyprland_model::ChangeGroupActiveArgs;
use smearor_hyprland_model::ChangeSplitRatioArgs;
use smearor_hyprland_model::CloseWindowArgs;
use smearor_hyprland_model::CreateWorkspaceArgs;
use smearor_hyprland_model::CustomDispatchArgs;
use smearor_hyprland_model::CycleWindowArgs;
use smearor_hyprland_model::ExecArgs;
use smearor_hyprland_model::ExitArgs;
use smearor_hyprland_model::FocusCurrentOrLastArgs;
use smearor_hyprland_model::FocusMasterArgs;
use smearor_hyprland_model::FocusMonitorArgs;
use smearor_hyprland_model::FocusUrgentOrLastArgs;
use smearor_hyprland_model::FocusWindowArgs;
use smearor_hyprland_model::ForceRendererReloadArgs;
use smearor_hyprland_model::FullscreenTypeArgs;
use smearor_hyprland_model::GlobalDispatchArgs;
use smearor_hyprland_model::KillActiveWindowArgs;
use smearor_hyprland_model::KillArgs;
use smearor_hyprland_model::LockGroupsArgs;
use smearor_hyprland_model::MoveActiveArgs;
use smearor_hyprland_model::MoveCurrentWorkspaceToMonitorArgs;
use smearor_hyprland_model::MoveCursorArgs;
use smearor_hyprland_model::MoveCursorToCornerArgs;
use smearor_hyprland_model::MoveFocusArgs;
use smearor_hyprland_model::MoveFocusedWindowToWorkspaceArgs;
use smearor_hyprland_model::MoveFocusedWindowToWorkspaceSilentArgs;
use smearor_hyprland_model::MoveIntoGroupArgs;
use smearor_hyprland_model::MoveOutOfGroupArgs;
use smearor_hyprland_model::MoveToWorkspaceSilentArgs;
use smearor_hyprland_model::MoveWindowArgs;
use smearor_hyprland_model::MoveWindowDispatchArgs;
use smearor_hyprland_model::MoveWindowPixelArgs;
use smearor_hyprland_model::NotifyArgs;
use smearor_hyprland_model::OrientationArgs;
use smearor_hyprland_model::OutputCreateArgs;
use smearor_hyprland_model::OutputRemoveArgs;
use smearor_hyprland_model::PassArgs;
use smearor_hyprland_model::PluginLoadArgs;
use smearor_hyprland_model::PluginUnloadArgs;
use smearor_hyprland_model::ReloadArgs;
use smearor_hyprland_model::RemoveMasterArgs;
use smearor_hyprland_model::RenameWorkspaceArgs;
use smearor_hyprland_model::ResizeActiveArgs;
use smearor_hyprland_model::ResizeWindowPixelArgs;
use smearor_hyprland_model::SetCursorArgs;
use smearor_hyprland_model::SetCursorCtlArgs;
use smearor_hyprland_model::SetErrorArgs;
use smearor_hyprland_model::SetPropArgs;
use smearor_hyprland_model::SpecialWorkspaceArgs;
use smearor_hyprland_model::SwapActiveWorkspacesArgs;
use smearor_hyprland_model::SwapWindowArgs;
use smearor_hyprland_model::SwapWithMasterArgs;
use smearor_hyprland_model::SwitchWorkspaceArgs;
use smearor_hyprland_model::SwitchWorkspaceCompositorArgs;
use smearor_hyprland_model::SwitchXkbLayoutArgs;
use smearor_hyprland_model::ToggleDpmsArgs;
use smearor_hyprland_model::ToggleFakeFullscreenArgs;
use smearor_hyprland_model::ToggleFloatingArgs;
use smearor_hyprland_model::ToggleGroupArgs;
use smearor_hyprland_model::ToggleOpaqueArgs;
use smearor_hyprland_model::TogglePinArgs;
use smearor_hyprland_model::TogglePseudoArgs;
use smearor_hyprland_model::ToggleSplitArgs;
use smearor_hyprland_model::WorkspaceOptionArgs;
use smearor_model_mcp::NoArgs;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::ToolAnnotations;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;

fn register_tool(broadcaster: &MessageBroadcasterInner, name: &str, description: &str, schema: &str) {
    broadcaster.broadcast_message_to_topic(RegisterToolMessage::new(name, description, schema).with_annotations(&ToolAnnotations::idempotent()));
}

fn register_resource(broadcaster: &MessageBroadcasterInner, uri: &str, name: &str, description: &str, mime_type: &str) {
    broadcaster.broadcast_message_to_topic(RegisterResourceMessage::new(uri, name, description, mime_type));
}

fn register_prompt_with_memory(
    broadcaster: &MessageBroadcasterInner,
    name: &str,
    description: &str,
    arguments_schema: &str,
    memory_query: &str,
    entity_filter: &str,
) {
    broadcaster.broadcast_message_to_topic(RegisterPromptMessage::with_memory(name, description, arguments_schema, memory_query, entity_filter));
}

impl McpCapabilitiesRegistrator for HyprlandService {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        register_resource(
            &broadcaster,
            "hyprland://state",
            "Hyprland State",
            "Current Hyprland compositor state: active window, fullscreen, keyboard layout, submap.",
            "application/json",
        );
        register_resource(
            &broadcaster,
            "hyprland://active-window",
            "Active Window",
            "Information about the currently focused window (class, title, workspace).",
            "application/json",
        );
        register_resource(
            &broadcaster,
            "hyprland://workspace-snapshot",
            "Workspace Snapshot",
            "Snapshot of all workspaces and their states.",
            "application/json",
        );
        register_resource(&broadcaster, "hyprland://workspaces", "Workspaces", "List of all workspaces.", "application/json");
        register_resource(&broadcaster, "hyprland://monitors", "Monitors", "List of all monitors.", "application/json");
        register_resource(&broadcaster, "hyprland://window-status", "Window Status", "Recent window status events.", "application/json");
        register_resource(
            &broadcaster,
            "hyprland://workspace-status",
            "Workspace Status",
            "Recent workspace status events.",
            "application/json",
        );
        register_resource(&broadcaster, "hyprland://group-status", "Group Status", "Recent group status events.", "application/json");
        register_resource(
            &broadcaster,
            "hyprland://layer-status",
            "Layer Status",
            "Recent layer shell status events.",
            "application/json",
        );
        register_resource(&broadcaster, "hyprland://system-status", "System Status", "Recent system status events.", "application/json");
        register_resource(
            &broadcaster,
            "hyprland://windows",
            "Windows",
            "List of all windows (clients) with class, title, workspace, floating, fullscreen, pinned, and active state.",
            "application/json",
        );

        register_tool(
            &broadcaster,
            "hyprland_window_center",
            "Center the active window on screen.",
            &serde_json::to_string(&schema_for!(CenterWindowArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_change_group_active",
            "Change active window in a group.",
            &serde_json::to_string(&schema_for!(ChangeGroupActiveArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_change_split_ratio",
            "Change the split ratio of the active window.",
            &serde_json::to_string(&schema_for!(ChangeSplitRatioArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_close",
            "Close a specific window.",
            &serde_json::to_string(&schema_for!(CloseWindowArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_cycle",
            "Cycle focus to the next or previous window.",
            &serde_json::to_string(&schema_for!(CycleWindowArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_exec",
            "Execute a command via Hyprland dispatch.",
            &serde_json::to_string(&schema_for!(ExecArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_focus_current_or_last",
            "Focus the current or last focused window.",
            &serde_json::to_string(&schema_for!(FocusCurrentOrLastArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_focus_master",
            "Focus the master window.",
            &serde_json::to_string(&schema_for!(FocusMasterArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_focus_monitor",
            "Focus a specific monitor.",
            &serde_json::to_string(&schema_for!(FocusMonitorArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_focus_urgent_or_last",
            "Focus the urgent or last focused window.",
            &serde_json::to_string(&schema_for!(FocusUrgentOrLastArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_focus_window",
            "Focus a specific window.",
            &serde_json::to_string(&schema_for!(FocusWindowArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_kill_active",
            "Kill the active window.",
            &serde_json::to_string(&schema_for!(KillActiveWindowArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_move_active",
            "Move the active window by pixel delta or to exact position.",
            &serde_json::to_string(&schema_for!(MoveActiveArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_move_cursor",
            "Move the cursor to specified coordinates.",
            &serde_json::to_string(&schema_for!(MoveCursorArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_move_cursor_to_corner",
            "Move the cursor to a corner of the active window.",
            &serde_json::to_string(&schema_for!(MoveCursorToCornerArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_move_focus",
            "Move focus in a direction.",
            &serde_json::to_string(&schema_for!(MoveFocusArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_move_into_group",
            "Move the active window into a group.",
            &serde_json::to_string(&schema_for!(MoveIntoGroupArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_move_window",
            "Move a window by direction or to a monitor.",
            &serde_json::to_string(&schema_for!(MoveWindowDispatchArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_move_window_pixel",
            "Move a specific window by pixel delta.",
            &serde_json::to_string(&schema_for!(MoveWindowPixelArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_resize_active",
            "Resize the active window.",
            &serde_json::to_string(&schema_for!(ResizeActiveArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_resize_window_pixel",
            "Resize a specific window by pixel delta.",
            &serde_json::to_string(&schema_for!(ResizeWindowPixelArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_swap",
            "Swap the active window with next or previous.",
            &serde_json::to_string(&schema_for!(SwapWindowArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_window_swap_with_master",
            "Swap the active window with the master.",
            &serde_json::to_string(&schema_for!(SwapWithMasterArgs)).unwrap_or_default(),
        );

        register_tool(
            &broadcaster,
            "hyprland_move_window",
            "Move the active window to a workspace by ID.",
            &serde_json::to_string(&schema_for!(MoveWindowArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_workspace_move_current_to_monitor",
            "Move the current workspace to a specific monitor.",
            &serde_json::to_string(&schema_for!(MoveCurrentWorkspaceToMonitorArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_workspace_move_focused_window",
            "Move the focused window to a workspace.",
            &serde_json::to_string(&schema_for!(MoveFocusedWindowToWorkspaceArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_workspace_move_focused_window_silent",
            "Move the focused window to a workspace silently.",
            &serde_json::to_string(&schema_for!(MoveFocusedWindowToWorkspaceSilentArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_workspace_move_to_workspace_silent",
            "Move the active window to a workspace silently.",
            &serde_json::to_string(&schema_for!(MoveToWorkspaceSilentArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_workspace_rename",
            "Rename a workspace.",
            &serde_json::to_string(&schema_for!(RenameWorkspaceArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_workspace_swap_active",
            "Swap active workspaces between two monitors.",
            &serde_json::to_string(&schema_for!(SwapActiveWorkspacesArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_workspace_toggle_special",
            "Toggle a special workspace.",
            &serde_json::to_string(&schema_for!(SpecialWorkspaceArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_workspace_option",
            "Set a workspace option.",
            &serde_json::to_string(&schema_for!(WorkspaceOptionArgs)).unwrap_or_default(),
        );

        register_tool(
            &broadcaster,
            "hyprland_toggle_floating",
            "Toggle floating mode for the active window.",
            &serde_json::to_string(&schema_for!(ToggleFloatingArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_toggle_fullscreen",
            "Toggle fullscreen for the active window.",
            &serde_json::to_string(&schema_for!(FullscreenTypeArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_toggle_dpms",
            "Toggle DPMS (display power management).",
            &serde_json::to_string(&schema_for!(ToggleDpmsArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_toggle_fake_fullscreen",
            "Toggle fake fullscreen.",
            &serde_json::to_string(&schema_for!(ToggleFakeFullscreenArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_toggle_group",
            "Toggle window group.",
            &serde_json::to_string(&schema_for!(ToggleGroupArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_toggle_opaque",
            "Toggle opaque.",
            &serde_json::to_string(&schema_for!(ToggleOpaqueArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_toggle_pin",
            "Toggle pin.",
            &serde_json::to_string(&schema_for!(TogglePinArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_toggle_pseudo",
            "Toggle pseudo tiling.",
            &serde_json::to_string(&schema_for!(TogglePseudoArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_toggle_split",
            "Toggle split.",
            &serde_json::to_string(&schema_for!(ToggleSplitArgs)).unwrap_or_default(),
        );

        register_tool(
            &broadcaster,
            "hyprland_system_add_master",
            "Add a master to the layout.",
            &serde_json::to_string(&schema_for!(AddMasterArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_system_bring_active_to_top",
            "Bring the active window to top.",
            &serde_json::to_string(&schema_for!(BringActiveToTopArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_system_custom",
            "Execute a custom Hyprland dispatch.",
            &serde_json::to_string(&schema_for!(CustomDispatchArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_system_exit",
            "Exit Hyprland.",
            &serde_json::to_string(&schema_for!(ExitArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_system_force_renderer_reload",
            "Force renderer reload.",
            &serde_json::to_string(&schema_for!(ForceRendererReloadArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_system_global",
            "Execute a global keybinding.",
            &serde_json::to_string(&schema_for!(GlobalDispatchArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_system_lock_groups",
            "Lock, unlock, or toggle group locks.",
            &serde_json::to_string(&schema_for!(LockGroupsArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_system_move_out_of_group",
            "Move the active window out of its group.",
            &serde_json::to_string(&schema_for!(MoveOutOfGroupArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_system_orientation",
            "Set window orientation.",
            &serde_json::to_string(&schema_for!(OrientationArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_system_pass",
            "Pass a key event to a window.",
            &serde_json::to_string(&schema_for!(PassArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_system_remove_master",
            "Remove a master from the layout.",
            &serde_json::to_string(&schema_for!(RemoveMasterArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_system_set_cursor",
            "Set the cursor theme and size.",
            &serde_json::to_string(&schema_for!(SetCursorArgs)).unwrap_or_default(),
        );

        register_tool(
            &broadcaster,
            "hyprland_ctl_kill",
            "Enter kill mode.",
            &serde_json::to_string(&schema_for!(KillArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_ctl_notify",
            "Send a Hyprland notification.",
            &serde_json::to_string(&schema_for!(NotifyArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_ctl_output_create",
            "Create a virtual output.",
            &serde_json::to_string(&schema_for!(OutputCreateArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_ctl_output_remove",
            "Remove a virtual output.",
            &serde_json::to_string(&schema_for!(OutputRemoveArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_ctl_plugin_load",
            "Load a Hyprland plugin.",
            &serde_json::to_string(&schema_for!(PluginLoadArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_ctl_plugin_unload",
            "Unload a Hyprland plugin.",
            &serde_json::to_string(&schema_for!(PluginUnloadArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_ctl_reload",
            "Reload Hyprland configuration.",
            &serde_json::to_string(&schema_for!(ReloadArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_ctl_set_cursor",
            "Set cursor (ctl variant).",
            &serde_json::to_string(&schema_for!(SetCursorCtlArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_ctl_set_error",
            "Set an error status.",
            &serde_json::to_string(&schema_for!(SetErrorArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_ctl_set_prop",
            "Set a window property.",
            &serde_json::to_string(&schema_for!(SetPropArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_ctl_switch_xkb_layout",
            "Switch the XKB keyboard layout.",
            &serde_json::to_string(&schema_for!(SwitchXkbLayoutArgs)).unwrap_or_default(),
        );

        register_tool(
            &broadcaster,
            "hyprland_switch_workspace",
            "Switch to a workspace by ID.",
            &serde_json::to_string(&schema_for!(SwitchWorkspaceArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_compositor_create_workspace",
            "Create a new workspace.",
            &serde_json::to_string(&schema_for!(CreateWorkspaceArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_compositor_switch_workspace",
            "Switch workspace (compositor-level).",
            &serde_json::to_string(&schema_for!(SwitchWorkspaceCompositorArgs)).unwrap_or_default(),
        );
        register_tool(
            &broadcaster,
            "hyprland_refresh_state",
            "Refresh the Hyprland state and workspace snapshot from the compositor. Use this before reading hyprland://state or hyprland://workspace-snapshot resources to ensure fresh data.",
            &serde_json::to_string(&schema_for!(NoArgs)).unwrap_or_default(),
        );

        let no_args_schema = serde_json::to_string(&schema_for!(NoArgs)).unwrap_or_default();

        register_prompt_with_memory(
            &broadcaster,
            "hyprland_overview",
            "Comprehensive guide of all Hyprland MCP tools, resources, and prompts. Use this when the LLM needs to understand the full scope of available compositor controls.",
            &no_args_schema,
            "Hyprland compositor configuration and usage preference",
            "workspace,window,monitor,group,system",
        );
        register_prompt_with_memory(
            &broadcaster,
            "hyprland_quick_reference",
            "Compact one-line-per-tool reference card for all Hyprland MCP operations. Use this when the LLM needs a quick lookup of available tools and resources.",
            &no_args_schema,
            "Hyprland quick tool reference preference",
            "workspace,window,monitor,group,system",
        );
        register_prompt_with_memory(
            &broadcaster,
            "hyprland_window_guide",
            "Detailed window management workflow guide with tool descriptions, argument examples, and step-by-step instructions. Use this when the user wants to manage windows (close, focus, move, resize, swap, group).",
            &no_args_schema,
            "Hyprland window management preference",
            "window,workspace",
        );
        register_prompt_with_memory(
            &broadcaster,
            "hyprland_workspace_guide",
            "Detailed workspace management workflow guide with tool descriptions, argument examples, and step-by-step instructions. Use this when the user wants to manage workspaces (switch, create, rename, move windows, swap, toggle special).",
            &no_args_schema,
            "Hyprland workspace layout preference",
            "workspace,monitor",
        );
    }
}
