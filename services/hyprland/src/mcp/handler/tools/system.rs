use crate::service::HyprlandCommand;
use crate::service::HyprlandService;
use smearor_hyprland_model::AddMasterArgs;
use smearor_hyprland_model::BringActiveToTopArgs;
use smearor_hyprland_model::CustomDispatchArgs;
use smearor_hyprland_model::ExitArgs;
use smearor_hyprland_model::ForceRendererReloadArgs;
use smearor_hyprland_model::GlobalDispatchArgs;
use smearor_hyprland_model::HyprlandMcpTools;
use smearor_hyprland_model::HyprlandSystemDispatchMessage;
use smearor_hyprland_model::LockGroupsArgs;
use smearor_hyprland_model::MoveOutOfGroupArgs;
use smearor_hyprland_model::OrientationArgs;
use smearor_hyprland_model::OrientationKind;
use smearor_hyprland_model::PassArgs;
use smearor_hyprland_model::RemoveMasterArgs;
use smearor_hyprland_model::SetCursorArgs;
use smearor_hyprland_model::SystemDispatchKind;
use smearor_hyprland_model::SystemDispatchOps;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;

/// Result of a system tool invocation.
struct SystemToolResult {
    /// Human-readable response message for the MCP client.
    response_message: &'static str,
}

impl HyprlandService {
    pub(crate) fn handle_system_tool(&self, tool: HyprlandMcpTools, arguments: &str, correlation_id: &str, broadcaster: &MessageBroadcasterInner) {
        let result = match tool {
            HyprlandMcpTools::SystemAddMaster => {
                let _args: AddMasterArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = SystemDispatchOps {
                    add_master: stabby::option::Option::Some(smearor_hyprland_model::AddMasterDispatchMessage),
                    ..SystemDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::SystemDispatch(HyprlandSystemDispatchMessage {
                    kind: SystemDispatchKind::AddMaster,
                    ops,
                }));
                SystemToolResult {
                    response_message: "Added master to layout",
                }
            }
            HyprlandMcpTools::SystemBringActiveToTop => {
                let _args: BringActiveToTopArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = SystemDispatchOps {
                    bring_active_to_top: stabby::option::Option::Some(smearor_hyprland_model::BringActiveToTopDispatchMessage),
                    ..SystemDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::SystemDispatch(HyprlandSystemDispatchMessage {
                    kind: SystemDispatchKind::BringActiveToTop,
                    ops,
                }));
                SystemToolResult {
                    response_message: "Brought active window to top",
                }
            }
            HyprlandMcpTools::SystemCustom => {
                let args: CustomDispatchArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = SystemDispatchOps {
                    custom: stabby::option::Option::Some(smearor_hyprland_model::CustomDispatchMessageStabby {
                        name: args.name.into(),
                        value: args.value.into(),
                    }),
                    ..SystemDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::SystemDispatch(HyprlandSystemDispatchMessage {
                    kind: SystemDispatchKind::Custom,
                    ops,
                }));
                SystemToolResult {
                    response_message: "Executed custom dispatch",
                }
            }
            HyprlandMcpTools::SystemExit => {
                let _args: ExitArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = SystemDispatchOps {
                    exit: stabby::option::Option::Some(smearor_hyprland_model::ExitDispatchMessage),
                    ..SystemDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::SystemDispatch(HyprlandSystemDispatchMessage {
                    kind: SystemDispatchKind::Exit,
                    ops,
                }));
                SystemToolResult {
                    response_message: "Exit dispatched",
                }
            }
            HyprlandMcpTools::SystemForceRendererReload => {
                let _args: ForceRendererReloadArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = SystemDispatchOps {
                    force_renderer_reload: stabby::option::Option::Some(smearor_hyprland_model::ForceRendererReloadDispatchMessage),
                    ..SystemDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::SystemDispatch(HyprlandSystemDispatchMessage {
                    kind: SystemDispatchKind::ForceRendererReload,
                    ops,
                }));
                SystemToolResult {
                    response_message: "Forced renderer reload",
                }
            }
            HyprlandMcpTools::SystemGlobal => {
                let args: GlobalDispatchArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = SystemDispatchOps {
                    global: stabby::option::Option::Some(smearor_hyprland_model::GlobalDispatchMessageStabby { key: args.key.into() }),
                    ..SystemDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::SystemDispatch(HyprlandSystemDispatchMessage {
                    kind: SystemDispatchKind::Global,
                    ops,
                }));
                SystemToolResult {
                    response_message: "Executed global keybinding",
                }
            }
            HyprlandMcpTools::SystemLockGroups => {
                let args: LockGroupsArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = SystemDispatchOps {
                    lock_groups: stabby::option::Option::Some(smearor_hyprland_model::LockGroupsDispatchMessage {
                        lock_type: args.lock_type.into(),
                    }),
                    ..SystemDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::SystemDispatch(HyprlandSystemDispatchMessage {
                    kind: SystemDispatchKind::LockGroups,
                    ops,
                }));
                SystemToolResult {
                    response_message: "Lock groups dispatched",
                }
            }
            HyprlandMcpTools::SystemMoveOutOfGroup => {
                let _args: MoveOutOfGroupArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = SystemDispatchOps {
                    move_out_of_group: stabby::option::Option::Some(smearor_hyprland_model::MoveOutOfGroupDispatchMessage),
                    ..SystemDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::SystemDispatch(HyprlandSystemDispatchMessage {
                    kind: SystemDispatchKind::MoveOutOfGroup,
                    ops,
                }));
                SystemToolResult {
                    response_message: "Moved active window out of group",
                }
            }
            HyprlandMcpTools::SystemOrientation => {
                let args: OrientationArgs = serde_json::from_str(arguments).unwrap_or_default();
                let (kind, ops) = match args.orientation {
                    OrientationKind::Bottom => (
                        SystemDispatchKind::OrientationBottom,
                        SystemDispatchOps {
                            orientation_bottom: stabby::option::Option::Some(smearor_hyprland_model::OrientationBottomDispatchMessage),
                            ..SystemDispatchOps::default()
                        },
                    ),
                    OrientationKind::Center => (
                        SystemDispatchKind::OrientationCenter,
                        SystemDispatchOps {
                            orientation_center: stabby::option::Option::Some(smearor_hyprland_model::OrientationCenterDispatchMessage),
                            ..SystemDispatchOps::default()
                        },
                    ),
                    OrientationKind::Left => (
                        SystemDispatchKind::OrientationLeft,
                        SystemDispatchOps {
                            orientation_left: stabby::option::Option::Some(smearor_hyprland_model::OrientationLeftDispatchMessage),
                            ..SystemDispatchOps::default()
                        },
                    ),
                    OrientationKind::Next => (
                        SystemDispatchKind::OrientationNext,
                        SystemDispatchOps {
                            orientation_next: stabby::option::Option::Some(smearor_hyprland_model::OrientationNextDispatchMessage),
                            ..SystemDispatchOps::default()
                        },
                    ),
                    OrientationKind::Prev => (
                        SystemDispatchKind::OrientationPrev,
                        SystemDispatchOps {
                            orientation_prev: stabby::option::Option::Some(smearor_hyprland_model::OrientationPrevDispatchMessage),
                            ..SystemDispatchOps::default()
                        },
                    ),
                    OrientationKind::Right => (
                        SystemDispatchKind::OrientationRight,
                        SystemDispatchOps {
                            orientation_right: stabby::option::Option::Some(smearor_hyprland_model::OrientationRightDispatchMessage),
                            ..SystemDispatchOps::default()
                        },
                    ),
                    OrientationKind::Top => (
                        SystemDispatchKind::OrientationTop,
                        SystemDispatchOps {
                            orientation_top: stabby::option::Option::Some(smearor_hyprland_model::OrientationTopDispatchMessage),
                            ..SystemDispatchOps::default()
                        },
                    ),
                };
                let _ = self
                    .command_sender
                    .send(HyprlandCommand::SystemDispatch(HyprlandSystemDispatchMessage { kind, ops }));
                SystemToolResult {
                    response_message: "Set window orientation",
                }
            }
            HyprlandMcpTools::SystemPass => {
                let args: PassArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = SystemDispatchOps {
                    pass: stabby::option::Option::Some(smearor_hyprland_model::PassDispatchMessage {
                        window_identifier: args.window_identifier.into(),
                    }),
                    ..SystemDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::SystemDispatch(HyprlandSystemDispatchMessage {
                    kind: SystemDispatchKind::Pass,
                    ops,
                }));
                SystemToolResult {
                    response_message: "Passed key event to window",
                }
            }
            HyprlandMcpTools::SystemRemoveMaster => {
                let _args: RemoveMasterArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = SystemDispatchOps {
                    remove_master: stabby::option::Option::Some(smearor_hyprland_model::RemoveMasterDispatchMessage),
                    ..SystemDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::SystemDispatch(HyprlandSystemDispatchMessage {
                    kind: SystemDispatchKind::RemoveMaster,
                    ops,
                }));
                SystemToolResult {
                    response_message: "Removed master from layout",
                }
            }
            HyprlandMcpTools::SystemSetCursor => {
                let args: SetCursorArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = SystemDispatchOps {
                    set_cursor: stabby::option::Option::Some(smearor_hyprland_model::SetCursorDispatchMessageStabby {
                        theme: args.theme.into(),
                        size: args.size,
                    }),
                    ..SystemDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::SystemDispatch(HyprlandSystemDispatchMessage {
                    kind: SystemDispatchKind::SetCursor,
                    ops,
                }));
                SystemToolResult {
                    response_message: "Set cursor theme and size",
                }
            }
            _ => return,
        };
        let response = InvokeToolResponse::success(correlation_id, result.response_message);
        broadcaster.broadcast_message_to_topic(response);
    }
}
