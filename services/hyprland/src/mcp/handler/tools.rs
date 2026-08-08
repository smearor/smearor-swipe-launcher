use crate::service::HyprlandCommand;
use crate::service::HyprlandService;
use smearor_hyprland_model::HyprlandMcpTools;
use smearor_hyprland_model::HyprlandToggleDispatchMessage;
use smearor_hyprland_model::HyprlandWorkspaceDispatchMessage;
use smearor_hyprland_model::HyprlandWorkspaceIdentifier;
use smearor_hyprland_model::MoveFocusedWindowToWorkspaceDispatchMessage;
use smearor_hyprland_model::MoveWindowArgs;
use smearor_hyprland_model::SwitchWorkspaceArgs;
use smearor_hyprland_model::ToggleDispatchKind;
use smearor_hyprland_model::ToggleDispatchOps;
use smearor_hyprland_model::ToggleFloatingDispatchMessage;
use smearor_hyprland_model::WorkspaceDispatchKind;
use smearor_hyprland_model::WorkspaceDispatchOps;
use smearor_model_compositor::SwitchWorkspaceMessage;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::trace;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        trace!("Hyprland Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match HyprlandMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &message.0.correlation_id)));
                return;
            }
        };
        match tool {
            HyprlandMcpTools::SwitchWorkspace => {
                let args: SwitchWorkspaceArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                let _ = self.command_sender.send(HyprlandCommand::SwitchWorkspace(SwitchWorkspaceMessage {
                    workspace_id: args.workspace_id,
                }));
                let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Switched to workspace {}", args.workspace_id));
                broadcaster.broadcast_message_to_topic(response);
            }
            HyprlandMcpTools::MoveWindowToWorkspace => {
                let args: MoveWindowArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                let dispatch_message = HyprlandWorkspaceDispatchMessage {
                    kind: WorkspaceDispatchKind::MoveFocusedWindowToWorkspace,
                    ops: WorkspaceDispatchOps {
                        move_focused_window_to_workspace: stabby::option::Option::Some(MoveFocusedWindowToWorkspaceDispatchMessage {
                            identifier: HyprlandWorkspaceIdentifier::Id(args.workspace_id),
                        }),
                        ..WorkspaceDispatchOps::default()
                    },
                };
                let _ = self.command_sender.send(HyprlandCommand::WorkspaceDispatch(dispatch_message));
                let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Moved window to workspace {}", args.workspace_id));
                broadcaster.broadcast_message_to_topic(response);
            }
            HyprlandMcpTools::ToggleFloating => {
                let toggle_message = HyprlandToggleDispatchMessage {
                    kind: ToggleDispatchKind::ToggleFloating,
                    ops: ToggleDispatchOps {
                        toggle_floating: stabby::option::Option::Some(ToggleFloatingDispatchMessage),
                        ..ToggleDispatchOps::default()
                    },
                };
                let _ = self.command_sender.send(HyprlandCommand::ToggleDispatch(toggle_message));
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Toggled floating mode");
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}
