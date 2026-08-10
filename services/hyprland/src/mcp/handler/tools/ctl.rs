use crate::service::HyprlandCommand;
use crate::service::HyprlandService;
use smearor_hyprland_model::HyprlandMcpTools;
use smearor_hyprland_model::KeywordGetArgs;
use smearor_hyprland_model::KeywordSetArgs;
use smearor_hyprland_model::KillArgs;
use smearor_hyprland_model::NotifyArgs;
use smearor_hyprland_model::OutputCreateArgs;
use smearor_hyprland_model::OutputRemoveArgs;
use smearor_hyprland_model::PluginLoadArgs;
use smearor_hyprland_model::PluginUnloadArgs;
use smearor_hyprland_model::ReloadArgs;
use smearor_hyprland_model::SendShortcutArgs;
use smearor_hyprland_model::SetCursorCtlArgs;
use smearor_hyprland_model::SetErrorArgs;
use smearor_hyprland_model::SetPropArgs;
use smearor_hyprland_model::SwitchXkbLayoutArgs;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;

/// Result of a control command tool invocation.
struct CtlToolResult {
    /// Human-readable response message for the MCP client.
    response_message: &'static str,
}

impl HyprlandService {
    pub(crate) fn handle_ctl_tool(&self, tool: HyprlandMcpTools, arguments: &str, correlation_id: &str, broadcaster: &MessageBroadcasterInner) {
        let result = match tool {
            HyprlandMcpTools::CtlKill => {
                let _args: KillArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self.command_sender.send(HyprlandCommand::CtlKill(smearor_hyprland_model::KillCommandMessage));
                CtlToolResult {
                    response_message: "Entered kill mode",
                }
            }
            HyprlandMcpTools::CtlNotify => {
                let args: NotifyArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self
                    .command_sender
                    .send(HyprlandCommand::CtlNotify(smearor_hyprland_model::NotifyCommandMessage {
                        icon: args.icon.into(),
                        time_ms: args.time_ms,
                        color: args.color.into(),
                        message: args.message,
                    }));
                CtlToolResult {
                    response_message: "Sent notification",
                }
            }
            HyprlandMcpTools::CtlOutputCreate => {
                let args: OutputCreateArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self
                    .command_sender
                    .send(HyprlandCommand::CtlOutputCreate(smearor_hyprland_model::OutputCreateCommandMessage {
                        backend: args.backend.into(),
                    }));
                CtlToolResult {
                    response_message: "Created virtual output",
                }
            }
            HyprlandMcpTools::CtlOutputRemove => {
                let args: OutputRemoveArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self
                    .command_sender
                    .send(HyprlandCommand::CtlOutputRemove(smearor_hyprland_model::OutputRemoveCommandMessage { name: args.name }));
                CtlToolResult {
                    response_message: "Removed virtual output",
                }
            }
            HyprlandMcpTools::CtlPluginLoad => {
                let args: PluginLoadArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self
                    .command_sender
                    .send(HyprlandCommand::CtlPluginLoad(smearor_hyprland_model::PluginLoadCommandMessage { path: args.path }));
                CtlToolResult {
                    response_message: "Loaded plugin",
                }
            }
            HyprlandMcpTools::CtlPluginUnload => {
                let args: PluginUnloadArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self
                    .command_sender
                    .send(HyprlandCommand::CtlPluginUnload(smearor_hyprland_model::PluginUnloadCommandMessage { name: args.name }));
                CtlToolResult {
                    response_message: "Unloaded plugin",
                }
            }
            HyprlandMcpTools::CtlReload => {
                let _args: ReloadArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self
                    .command_sender
                    .send(HyprlandCommand::CtlReload(smearor_hyprland_model::ReloadCommandMessage));
                CtlToolResult {
                    response_message: "Reloaded Hyprland configuration",
                }
            }
            HyprlandMcpTools::CtlSetCursor => {
                let args: SetCursorCtlArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self
                    .command_sender
                    .send(HyprlandCommand::CtlSetCursor(smearor_hyprland_model::SetCursorCommandMessage {
                        theme: args.theme,
                        size: args.size,
                    }));
                CtlToolResult {
                    response_message: "Set cursor (ctl)",
                }
            }
            HyprlandMcpTools::CtlSetError => {
                let args: SetErrorArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self
                    .command_sender
                    .send(HyprlandCommand::CtlSetError(smearor_hyprland_model::SetErrorCommandMessage {
                        color: args.color.into(),
                        message: args.message,
                    }));
                CtlToolResult {
                    response_message: "Set error status",
                }
            }
            HyprlandMcpTools::CtlSetProp => {
                let args: SetPropArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self
                    .command_sender
                    .send(HyprlandCommand::CtlSetProp(smearor_hyprland_model::SetPropCommandMessage {
                        identifier: args.identifier,
                        prop: args.prop.into(),
                        lock: args.lock,
                    }));
                CtlToolResult {
                    response_message: "Set window property",
                }
            }
            HyprlandMcpTools::CtlSwitchXkbLayout => {
                let args: SwitchXkbLayoutArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self
                    .command_sender
                    .send(HyprlandCommand::CtlSwitchXkbLayout(smearor_hyprland_model::SwitchXkbLayoutCommandMessage {
                        device: args.device,
                        cmd: args.cmd.into(),
                    }));
                CtlToolResult {
                    response_message: "Switched XKB keyboard layout",
                }
            }
            HyprlandMcpTools::CtlKeywordSet => {
                let args: KeywordSetArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self
                    .command_sender
                    .send(HyprlandCommand::CtlKeywordSet(smearor_hyprland_model::KeywordSetCommandMessage {
                        keyword: args.keyword,
                        value: args.value,
                    }));
                CtlToolResult {
                    response_message: "Set keyword",
                }
            }
            HyprlandMcpTools::CtlKeywordGet => {
                let args: KeywordGetArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self
                    .command_sender
                    .send(HyprlandCommand::CtlKeywordGet(smearor_hyprland_model::KeywordGetCommandMessage {
                        correlation_id: correlation_id.to_string(),
                        keyword: args.keyword,
                    }));
                return;
            }
            HyprlandMcpTools::CtlSendShortcut => {
                let args: SendShortcutArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self
                    .command_sender
                    .send(HyprlandCommand::CtlSendShortcut(smearor_hyprland_model::SendShortcutCommandMessage {
                        mods: args.mods,
                        key: args.key,
                        window: args.window,
                    }));
                CtlToolResult {
                    response_message: "Sent shortcut",
                }
            }
            _ => return,
        };
        let response = InvokeToolResponse::success(correlation_id, result.response_message);
        broadcaster.broadcast_message_to_topic(response);
    }
}
