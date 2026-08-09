use crate::service::HyprlandCommand;
use crate::service::HyprlandService;
use smearor_hyprland_model::HyprlandMcpTools;
use smearor_hyprland_model::HyprlandWorkspaceDispatchMessage;
use smearor_hyprland_model::MoveCurrentWorkspaceToMonitorArgs;
use smearor_hyprland_model::MoveFocusedWindowToWorkspaceArgs;
use smearor_hyprland_model::MoveFocusedWindowToWorkspaceSilentArgs;
use smearor_hyprland_model::MoveToWorkspaceSilentArgs;
use smearor_hyprland_model::MoveWindowArgs;
use smearor_hyprland_model::RenameWorkspaceArgs;
use smearor_hyprland_model::SpecialWorkspaceArgs;
use smearor_hyprland_model::SwapActiveWorkspacesArgs;
use smearor_hyprland_model::WorkspaceDispatchKind;
use smearor_hyprland_model::WorkspaceDispatchOps;
use smearor_hyprland_model::WorkspaceOptionArgs;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;

/// Result of a workspace tool invocation.
struct WorkspaceToolResult {
    /// Human-readable response message for the MCP client.
    response_message: &'static str,
}

impl HyprlandService {
    pub(crate) fn handle_workspace_tool(&self, tool: HyprlandMcpTools, arguments: &str, correlation_id: &str, broadcaster: &MessageBroadcasterInner) {
        let result = match tool {
            HyprlandMcpTools::MoveWindowToWorkspace => {
                let args: MoveWindowArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WorkspaceDispatchOps {
                    move_to_workspace: stabby::option::Option::Some(smearor_hyprland_model::MoveToWorkspaceDispatchMessage {
                        identifier: smearor_hyprland_model::HyprlandWorkspaceIdentifierWithSpecial {
                            kind: smearor_hyprland_model::HyprlandWorkspaceIdentifierKind::Id,
                            id: args.workspace_id,
                            ..Default::default()
                        },
                    }),
                    ..WorkspaceDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WorkspaceDispatch(HyprlandWorkspaceDispatchMessage {
                    kind: WorkspaceDispatchKind::MoveToWorkspace,
                    ops,
                }));
                WorkspaceToolResult {
                    response_message: "Moved active window to workspace",
                }
            }
            HyprlandMcpTools::WorkspaceMoveCurrentToMonitor => {
                let args: MoveCurrentWorkspaceToMonitorArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WorkspaceDispatchOps {
                    move_current_workspace_to_monitor: stabby::option::Option::Some(smearor_hyprland_model::MoveCurrentWorkspaceToMonitorDispatchMessage {
                        monitor_identifier: args.monitor_identifier.into(),
                    }),
                    ..WorkspaceDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WorkspaceDispatch(HyprlandWorkspaceDispatchMessage {
                    kind: WorkspaceDispatchKind::MoveCurrentWorkspaceToMonitor,
                    ops,
                }));
                WorkspaceToolResult {
                    response_message: "Moved current workspace to monitor",
                }
            }
            HyprlandMcpTools::WorkspaceMoveFocusedWindow => {
                let args: MoveFocusedWindowToWorkspaceArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WorkspaceDispatchOps {
                    move_focused_window_to_workspace: stabby::option::Option::Some(smearor_hyprland_model::MoveFocusedWindowToWorkspaceDispatchMessage {
                        identifier: args.identifier.into(),
                    }),
                    ..WorkspaceDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WorkspaceDispatch(HyprlandWorkspaceDispatchMessage {
                    kind: WorkspaceDispatchKind::MoveFocusedWindowToWorkspace,
                    ops,
                }));
                WorkspaceToolResult {
                    response_message: "Moved focused window to workspace",
                }
            }
            HyprlandMcpTools::WorkspaceMoveFocusedWindowSilent => {
                let args: MoveFocusedWindowToWorkspaceSilentArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WorkspaceDispatchOps {
                    move_focused_window_to_workspace_silent: stabby::option::Option::Some(
                        smearor_hyprland_model::MoveFocusedWindowToWorkspaceSilentDispatchMessageStabby {
                            identifier: args.identifier.into(),
                        },
                    ),
                    ..WorkspaceDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WorkspaceDispatch(HyprlandWorkspaceDispatchMessage {
                    kind: WorkspaceDispatchKind::MoveFocusedWindowToWorkspaceSilent,
                    ops,
                }));
                WorkspaceToolResult {
                    response_message: "Moved focused window to workspace silently",
                }
            }
            HyprlandMcpTools::WorkspaceMoveToWorkspaceSilent => {
                let args: MoveToWorkspaceSilentArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WorkspaceDispatchOps {
                    move_to_workspace_silent: stabby::option::Option::Some(smearor_hyprland_model::MoveToWorkspaceSilentDispatchMessageStabby {
                        identifier: args.identifier.into(),
                        window_identifier: args.window_identifier.map(Into::into).into(),
                    }),
                    ..WorkspaceDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WorkspaceDispatch(HyprlandWorkspaceDispatchMessage {
                    kind: WorkspaceDispatchKind::MoveToWorkspaceSilent,
                    ops,
                }));
                WorkspaceToolResult {
                    response_message: "Moved active window to workspace silently",
                }
            }
            HyprlandMcpTools::WorkspaceRename => {
                let args: RenameWorkspaceArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WorkspaceDispatchOps {
                    rename_workspace: stabby::option::Option::Some(smearor_hyprland_model::RenameWorkspaceDispatchMessageStabby {
                        workspace_id: args.workspace_id,
                        new_name: args.new_name.map(stabby::string::String::from).into(),
                    }),
                    ..WorkspaceDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WorkspaceDispatch(HyprlandWorkspaceDispatchMessage {
                    kind: WorkspaceDispatchKind::RenameWorkspace,
                    ops,
                }));
                WorkspaceToolResult {
                    response_message: "Renamed workspace",
                }
            }
            HyprlandMcpTools::WorkspaceSwapActive => {
                let args: SwapActiveWorkspacesArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WorkspaceDispatchOps {
                    swap_active_workspaces: stabby::option::Option::Some(smearor_hyprland_model::SwapActiveWorkspacesDispatchMessage {
                        monitor_a: args.monitor_a.into(),
                        monitor_b: args.monitor_b.into(),
                    }),
                    ..WorkspaceDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WorkspaceDispatch(HyprlandWorkspaceDispatchMessage {
                    kind: WorkspaceDispatchKind::SwapActiveWorkspaces,
                    ops,
                }));
                WorkspaceToolResult {
                    response_message: "Swapped active workspaces between monitors",
                }
            }
            HyprlandMcpTools::WorkspaceToggleSpecial => {
                let args: SpecialWorkspaceArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WorkspaceDispatchOps {
                    toggle_special_workspace: stabby::option::Option::Some(smearor_hyprland_model::ToggleSpecialWorkspaceDispatchMessageStabby {
                        workspace_name: args.workspace_name.map(stabby::string::String::from).into(),
                    }),
                    ..WorkspaceDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WorkspaceDispatch(HyprlandWorkspaceDispatchMessage {
                    kind: WorkspaceDispatchKind::ToggleSpecialWorkspace,
                    ops,
                }));
                WorkspaceToolResult {
                    response_message: "Toggled special workspace",
                }
            }
            HyprlandMcpTools::WorkspaceOption => {
                let args: WorkspaceOptionArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WorkspaceDispatchOps {
                    workspace_option: stabby::option::Option::Some(smearor_hyprland_model::WorkspaceOptionDispatchMessage { option: args.option.into() }),
                    ..WorkspaceDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WorkspaceDispatch(HyprlandWorkspaceDispatchMessage {
                    kind: WorkspaceDispatchKind::WorkspaceOption,
                    ops,
                }));
                WorkspaceToolResult {
                    response_message: "Set workspace option",
                }
            }
            _ => return,
        };
        let response = InvokeToolResponse::success(correlation_id, result.response_message);
        broadcaster.broadcast_message_to_topic(response);
    }
}
