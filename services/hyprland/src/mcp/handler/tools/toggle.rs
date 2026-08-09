use crate::service::HyprlandCommand;
use crate::service::HyprlandService;
use smearor_hyprland_model::FullscreenTypeArgs;
use smearor_hyprland_model::HyprlandMcpTools;
use smearor_hyprland_model::HyprlandToggleDispatchMessage;
use smearor_hyprland_model::ToggleDispatchKind;
use smearor_hyprland_model::ToggleDispatchOps;
use smearor_hyprland_model::ToggleDpmsArgs;
use smearor_hyprland_model::ToggleFakeFullscreenArgs;
use smearor_hyprland_model::ToggleGroupArgs;
use smearor_hyprland_model::ToggleOpaqueArgs;
use smearor_hyprland_model::TogglePinArgs;
use smearor_hyprland_model::TogglePseudoArgs;
use smearor_hyprland_model::ToggleSplitArgs;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;

/// Result of a toggle tool invocation.
struct ToggleToolResult {
    /// Human-readable response message for the MCP client.
    response_message: &'static str,
}

impl HyprlandService {
    pub(crate) fn handle_toggle_tool(&self, tool: HyprlandMcpTools, arguments: &str, correlation_id: &str, broadcaster: &MessageBroadcasterInner) {
        let result = match tool {
            HyprlandMcpTools::ToggleFloating => {
                let _args: smearor_hyprland_model::ToggleFloatingArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = ToggleDispatchOps {
                    toggle_floating: stabby::option::Option::Some(smearor_hyprland_model::ToggleFloatingDispatchMessage),
                    ..ToggleDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::ToggleDispatch(HyprlandToggleDispatchMessage {
                    kind: ToggleDispatchKind::ToggleFloating,
                    ops,
                }));
                ToggleToolResult {
                    response_message: "Toggled floating mode",
                }
            }
            HyprlandMcpTools::ToggleFullscreen => {
                let args: FullscreenTypeArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = ToggleDispatchOps {
                    toggle_fullscreen: stabby::option::Option::Some(smearor_hyprland_model::ToggleFullscreenDispatchMessage {
                        fullscreen_type: args.fullscreen_type.into(),
                    }),
                    ..ToggleDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::ToggleDispatch(HyprlandToggleDispatchMessage {
                    kind: ToggleDispatchKind::ToggleFullscreen,
                    ops,
                }));
                ToggleToolResult {
                    response_message: "Toggled fullscreen",
                }
            }
            HyprlandMcpTools::ToggleDpms => {
                let args: ToggleDpmsArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = ToggleDispatchOps {
                    toggle_dpms: stabby::option::Option::Some(smearor_hyprland_model::ToggleDpmsDispatchMessageStabby {
                        on: args.on,
                        name: args.name.map(stabby::string::String::from).into(),
                    }),
                    ..ToggleDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::ToggleDispatch(HyprlandToggleDispatchMessage {
                    kind: ToggleDispatchKind::ToggleDpms,
                    ops,
                }));
                ToggleToolResult {
                    response_message: "Toggled DPMS",
                }
            }
            HyprlandMcpTools::ToggleFakeFullscreen => {
                let _args: ToggleFakeFullscreenArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = ToggleDispatchOps {
                    toggle_fake_fullscreen: stabby::option::Option::Some(smearor_hyprland_model::ToggleFakeFullscreenDispatchMessage),
                    ..ToggleDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::ToggleDispatch(HyprlandToggleDispatchMessage {
                    kind: ToggleDispatchKind::ToggleFakeFullscreen,
                    ops,
                }));
                ToggleToolResult {
                    response_message: "Toggled fake fullscreen",
                }
            }
            HyprlandMcpTools::ToggleGroup => {
                let _args: ToggleGroupArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = ToggleDispatchOps {
                    toggle_group: stabby::option::Option::Some(smearor_hyprland_model::ToggleGroupDispatchMessage),
                    ..ToggleDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::ToggleDispatch(HyprlandToggleDispatchMessage {
                    kind: ToggleDispatchKind::ToggleGroup,
                    ops,
                }));
                ToggleToolResult {
                    response_message: "Toggled group",
                }
            }
            HyprlandMcpTools::ToggleOpaque => {
                let _args: ToggleOpaqueArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = ToggleDispatchOps {
                    toggle_opaque: stabby::option::Option::Some(smearor_hyprland_model::ToggleOpaqueDispatchMessage),
                    ..ToggleDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::ToggleDispatch(HyprlandToggleDispatchMessage {
                    kind: ToggleDispatchKind::ToggleOpaque,
                    ops,
                }));
                ToggleToolResult {
                    response_message: "Toggled opaque",
                }
            }
            HyprlandMcpTools::TogglePin => {
                let _args: TogglePinArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = ToggleDispatchOps {
                    toggle_pin: stabby::option::Option::Some(smearor_hyprland_model::TogglePinDispatchMessage),
                    ..ToggleDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::ToggleDispatch(HyprlandToggleDispatchMessage {
                    kind: ToggleDispatchKind::TogglePin,
                    ops,
                }));
                ToggleToolResult {
                    response_message: "Toggled pin",
                }
            }
            HyprlandMcpTools::TogglePseudo => {
                let _args: TogglePseudoArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = ToggleDispatchOps {
                    toggle_pseudo: stabby::option::Option::Some(smearor_hyprland_model::TogglePseudoDispatchMessage),
                    ..ToggleDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::ToggleDispatch(HyprlandToggleDispatchMessage {
                    kind: ToggleDispatchKind::TogglePseudo,
                    ops,
                }));
                ToggleToolResult {
                    response_message: "Toggled pseudo tiling",
                }
            }
            HyprlandMcpTools::ToggleSplit => {
                let _args: ToggleSplitArgs = serde_json::from_str(arguments).unwrap_or_default();
                let ops = ToggleDispatchOps {
                    toggle_split: stabby::option::Option::Some(smearor_hyprland_model::ToggleSplitDispatchMessage),
                    ..ToggleDispatchOps::default()
                };
                let _ = self.command_sender.send(HyprlandCommand::ToggleDispatch(HyprlandToggleDispatchMessage {
                    kind: ToggleDispatchKind::ToggleSplit,
                    ops,
                }));
                ToggleToolResult {
                    response_message: "Toggled split",
                }
            }
            _ => return,
        };
        let response = InvokeToolResponse::success(correlation_id, result.response_message);
        broadcaster.broadcast_message_to_topic(response);
    }
}
