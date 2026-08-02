use crate::service::TerminalCommandService;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_terminal_command_model::TerminalCommandMcpResources;

impl McpResourceHandler<TerminalCommandMcpResources> for TerminalCommandService {
    fn get_response(&self, request: &ResourceRequest<TerminalCommandMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        match request.resource {
            TerminalCommandMcpResources::Running => {
                let snapshot = self.running_commands_snapshot();
                let json = serde_json::json!({
                    "running_commands": snapshot.iter().map(|(command_id, pids, terminate_on_exit)| {
                        serde_json::json!({
                            "command_id": command_id,
                            "pids": pids,
                            "terminate_on_exit": terminate_on_exit,
                        })
                    }).collect::<Vec<_>>(),
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            TerminalCommandMcpResources::Configured => {
                let snapshot = self.configured_commands_snapshot();
                let json = serde_json::json!({
                    "configured_commands": snapshot.iter().map(|(command_id, command, args, restart_on_exit)| {
                        serde_json::json!({
                            "command_id": command_id,
                            "command": command,
                            "args": args,
                            "restart_on_exit": restart_on_exit,
                        })
                    }).collect::<Vec<_>>(),
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for TerminalCommandService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        self.handle_invoke_resource_message(message, sender_id);
    }
}
