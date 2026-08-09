use crate::service::HyprlandCommand;
use crate::service::HyprlandService;
use smearor_hyprland_model::CenterWindowArgs;
use smearor_hyprland_model::ChangeGroupActiveArgs;
use smearor_hyprland_model::ChangeSplitRatioArgs;
use smearor_hyprland_model::CloseWindowArgs;
use smearor_hyprland_model::CycleWindowArgs;
use smearor_hyprland_model::ExecArgs;
use smearor_hyprland_model::FocusCurrentOrLastArgs;
use smearor_hyprland_model::FocusMasterArgs;
use smearor_hyprland_model::FocusMonitorArgs;
use smearor_hyprland_model::FocusUrgentOrLastArgs;
use smearor_hyprland_model::FocusWindowArgs;
use smearor_hyprland_model::HyprlandMcpTools;
use smearor_hyprland_model::HyprlandWindowDispatchMessage;
use smearor_hyprland_model::KillActiveWindowArgs;
use smearor_hyprland_model::MoveActiveArgs;
use smearor_hyprland_model::MoveCursorArgs;
use smearor_hyprland_model::MoveCursorToCornerArgs;
use smearor_hyprland_model::MoveFocusArgs;
use smearor_hyprland_model::MoveIntoGroupArgs;
use smearor_hyprland_model::MoveWindowDispatchArgs;
use smearor_hyprland_model::MoveWindowPixelArgs;
use smearor_hyprland_model::ResizeActiveArgs;
use smearor_hyprland_model::ResizeWindowPixelArgs;
use smearor_hyprland_model::SwapWindowArgs;
use smearor_hyprland_model::SwapWithMasterArgs;
use smearor_hyprland_model::WindowDispatchKind;
use smearor_hyprland_model::WindowDispatchOps;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;

/// Result of a window tool invocation.
struct WindowToolResult {
    /// Human-readable response message for the MCP client.
    response_message: &'static str,
}

impl HyprlandService {
    pub(crate) fn handle_window_tool(&self, tool: HyprlandMcpTools, arguments: &str, correlation_id: &str, broadcaster: &MessageBroadcasterInner) {
        let result = match tool {
            HyprlandMcpTools::WindowCenter => {
                let _args: CenterWindowArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::CenterWindow,
                    ops: WindowDispatchOps::default(),
                }));
                WindowToolResult {
                    response_message: "Centered active window",
                }
            }
            HyprlandMcpTools::WindowChangeGroupActive => {
                let args: ChangeGroupActiveArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    change_group_active: stabby::option::Option::Some(smearor_hyprland_model::ChangeGroupActiveDispatchMessage {
                        direction: args.direction.into(),
                    }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::ChangeGroupActive,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Changed active window in group",
                }
            }
            HyprlandMcpTools::WindowChangeSplitRatio => {
                let args: ChangeSplitRatioArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    change_split_ratio: stabby::option::Option::Some(smearor_hyprland_model::ChangeSplitRatioDispatchMessage { ratio: args.ratio }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::ChangeSplitRatio,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Changed split ratio",
                }
            }
            HyprlandMcpTools::WindowClose => {
                let args: CloseWindowArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    close_window: stabby::option::Option::Some(smearor_hyprland_model::CloseWindowDispatchMessage {
                        window_identifier: args.window_identifier.into(),
                    }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::CloseWindow,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Closed window",
                }
            }
            HyprlandMcpTools::WindowCycle => {
                let args: CycleWindowArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    cycle_window: stabby::option::Option::Some(smearor_hyprland_model::CycleWindowDispatchMessage {
                        cycle_direction: args.cycle_direction.into(),
                    }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::CycleWindow,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Cycled window focus",
                }
            }
            HyprlandMcpTools::WindowExec => {
                let args: ExecArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    exec: stabby::option::Option::Some(smearor_hyprland_model::ExecDispatchMessageStabby { command: args.command.into() }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::Exec,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Executed command",
                }
            }
            HyprlandMcpTools::WindowFocusCurrentOrLast => {
                let _args: FocusCurrentOrLastArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::FocusCurrentOrLast,
                    ops: WindowDispatchOps::default(),
                }));
                WindowToolResult {
                    response_message: "Focused current or last window",
                }
            }
            HyprlandMcpTools::WindowFocusMaster => {
                let args: FocusMasterArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    focus_master: stabby::option::Option::Some(smearor_hyprland_model::FocusMasterDispatchMessage { param: args.param.into() }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::FocusMaster,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Focused master window",
                }
            }
            HyprlandMcpTools::WindowFocusMonitor => {
                let args: FocusMonitorArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    focus_monitor: stabby::option::Option::Some(smearor_hyprland_model::FocusMonitorDispatchMessage {
                        monitor_identifier: args.monitor_identifier.into(),
                    }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::FocusMonitor,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Focused monitor",
                }
            }
            HyprlandMcpTools::WindowFocusUrgentOrLast => {
                let _args: FocusUrgentOrLastArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::FocusUrgentOrLast,
                    ops: WindowDispatchOps::default(),
                }));
                WindowToolResult {
                    response_message: "Focused urgent or last window",
                }
            }
            HyprlandMcpTools::WindowFocusWindow => {
                let args: FocusWindowArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    focus_window: stabby::option::Option::Some(smearor_hyprland_model::FocusWindowDispatchMessage {
                        window_identifier: args.window_identifier.into(),
                    }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::FocusWindow,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Focused window",
                }
            }
            HyprlandMcpTools::WindowKillActive => {
                let _args: KillActiveWindowArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::KillActiveWindow,
                    ops: WindowDispatchOps::default(),
                }));
                WindowToolResult {
                    response_message: "Killed active window",
                }
            }
            HyprlandMcpTools::WindowMoveActive => {
                let args: MoveActiveArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    move_active: stabby::option::Option::Some(smearor_hyprland_model::MoveActiveDispatchMessage {
                        position: args.position.into(),
                    }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::MoveActive,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Moved active window",
                }
            }
            HyprlandMcpTools::WindowMoveCursor => {
                let args: MoveCursorArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    move_cursor: stabby::option::Option::Some(smearor_hyprland_model::MoveCursorDispatchMessage { x: args.x, y: args.y }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::MoveCursor,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Moved cursor",
                }
            }
            HyprlandMcpTools::WindowMoveCursorToCorner => {
                let args: MoveCursorToCornerArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    move_cursor_to_corner: stabby::option::Option::Some(smearor_hyprland_model::MoveCursorToCornerDispatchMessage {
                        corner: args.corner.into(),
                    }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::MoveCursorToCorner,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Moved cursor to corner",
                }
            }
            HyprlandMcpTools::WindowMoveFocus => {
                let args: MoveFocusArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    move_focus: stabby::option::Option::Some(smearor_hyprland_model::MoveFocusDispatchMessage {
                        direction: args.direction.into(),
                    }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::MoveFocus,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Moved focus",
                }
            }
            HyprlandMcpTools::WindowMoveIntoGroup => {
                let args: MoveIntoGroupArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    move_into_group: stabby::option::Option::Some(smearor_hyprland_model::MoveIntoGroupDispatchMessage {
                        direction: args.direction.into(),
                    }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::MoveIntoGroup,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Moved active window into group",
                }
            }
            HyprlandMcpTools::WindowMoveWindow => {
                let args: MoveWindowDispatchArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    move_window: stabby::option::Option::Some(smearor_hyprland_model::MoveWindowDispatchMessage {
                        window_move: args.window_move.into(),
                    }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::MoveWindow,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Moved window",
                }
            }
            HyprlandMcpTools::WindowMoveWindowPixel => {
                let args: MoveWindowPixelArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    move_window_pixel: stabby::option::Option::Some(smearor_hyprland_model::MoveWindowPixelDispatchMessage {
                        position: args.position.into(),
                        window_identifier: args.window_identifier.into(),
                    }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::MoveWindowPixel,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Moved window by pixels",
                }
            }
            HyprlandMcpTools::WindowResizeActive => {
                let args: ResizeActiveArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    resize_active: stabby::option::Option::Some(smearor_hyprland_model::ResizeActiveDispatchMessage {
                        position: args.position.into(),
                    }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::ResizeActive,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Resized active window",
                }
            }
            HyprlandMcpTools::WindowResizeWindowPixel => {
                let args: ResizeWindowPixelArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    resize_window_pixel: stabby::option::Option::Some(smearor_hyprland_model::ResizeWindowPixelDispatchMessage {
                        position: args.position.into(),
                        window_identifier: args.window_identifier.into(),
                    }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::ResizeWindowPixel,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Resized window by pixels",
                }
            }
            HyprlandMcpTools::WindowSwap => {
                let args: SwapWindowArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    swap_window: stabby::option::Option::Some(smearor_hyprland_model::SwapWindowDispatchMessage {
                        cycle_direction: args.cycle_direction.into(),
                    }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::SwapWindow,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Swapped window",
                }
            }
            HyprlandMcpTools::WindowSwapWithMaster => {
                let args: SwapWithMasterArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = WindowDispatchOps {
                    swap_with_master: stabby::option::Option::Some(smearor_hyprland_model::SwapWithMasterDispatchMessage { param: args.param.into() }),
                    ..WindowDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(HyprlandWindowDispatchMessage {
                    kind: WindowDispatchKind::SwapWithMaster,
                    ops,
                }));
                WindowToolResult {
                    response_message: "Swapped with master",
                }
            }
            _ => return,
        };
        let response = InvokeToolResponse::success(correlation_id, result.response_message);
        broadcaster.broadcast_message_to_topic(response);
    }
}
