use crate::service::HyprlandCommand;
use crate::service::HyprlandService;
use smearor_hyprland_model::CreateWorkspaceArgs;
use smearor_hyprland_model::HyprlandMcpTools;
use smearor_hyprland_model::SwitchWorkspaceArgs;
use smearor_hyprland_model::SwitchWorkspaceCompositorArgs;
use smearor_model_compositor::CreateWorkspaceMessage;
use smearor_model_compositor::SwitchWorkspaceMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;

/// Result of a compositor tool invocation.
struct CompositorToolResult {
    /// Human-readable response message for the MCP client.
    response_message: &'static str,
}

impl HyprlandService {
    pub(crate) fn handle_compositor_tool(&self, tool: HyprlandMcpTools, arguments: &str, correlation_id: &str, broadcaster: &MessageBroadcasterInner) {
        let result = match tool {
            HyprlandMcpTools::SwitchWorkspace => {
                let args: SwitchWorkspaceArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self.command_sender.send(HyprlandCommand::SwitchWorkspace(SwitchWorkspaceMessage {
                    workspace_id: args.workspace_id,
                }));
                CompositorToolResult {
                    response_message: "Switched workspace",
                }
            }
            HyprlandMcpTools::CompositorCreateWorkspace => {
                let args: CreateWorkspaceArgs = serde_json::from_str(arguments).unwrap_or_default();
                let position = match args.position {
                    smearor_hyprland_model::WorkspaceCreatePosition::Before => smearor_model_compositor::WorkspaceCreatePosition::Before,
                    smearor_hyprland_model::WorkspaceCreatePosition::After => smearor_model_compositor::WorkspaceCreatePosition::After,
                };
                let _ = self.command_sender.send(HyprlandCommand::CreateWorkspace(CreateWorkspaceMessage {
                    relative_to: args.relative_to,
                    position,
                }));
                CompositorToolResult {
                    response_message: "Created workspace",
                }
            }
            HyprlandMcpTools::CompositorSwitchWorkspace => {
                let args: SwitchWorkspaceCompositorArgs = serde_json::from_str(arguments).unwrap_or_default();
                let _ = self.command_sender.send(HyprlandCommand::SwitchWorkspace(SwitchWorkspaceMessage {
                    workspace_id: args.workspace_id,
                }));
                CompositorToolResult {
                    response_message: "Switched workspace",
                }
            }
            _ => return,
        };
        let response = InvokeToolResponse::success(correlation_id, result.response_message);
        broadcaster.broadcast_message_to_topic(response);
    }
}
