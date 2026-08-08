use crate::service::WaylandCommand;
use crate::service::WaylandWorkspaceService;
use smearor_model_compositor::CompositorMcpTools;
use smearor_model_compositor::SwitchWorkspaceArgs;
use smearor_model_compositor::SwitchWorkspaceMessage;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for WaylandWorkspaceService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("Wayland Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match CompositorMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &message.0.correlation_id)));
                return;
            }
        };
        match tool {
            CompositorMcpTools::SwitchWorkspace => {
                let args: SwitchWorkspaceArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                let _ = self.command_sender.send(WaylandCommand::SwitchWorkspace(SwitchWorkspaceMessage {
                    workspace_id: args.workspace_id,
                }));
                let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Switched to workspace {}", args.workspace_id));
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}
