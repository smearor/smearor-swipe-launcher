use crate::service::TerminalCommandService;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_terminal_command_model::TerminalCommandMcpTools;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for TerminalCommandService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("TerminalCommand Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match TerminalCommandMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &message.0.correlation_id)));
                return;
            }
        };
        match tool {
            TerminalCommandMcpTools::Launch => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let command_id = args.get("command_id").and_then(|v| v.as_str());
                match command_id {
                    Some(id) => {
                        let forked = args.get("forked").and_then(|v| v.as_bool()).unwrap_or(false);
                        let terminate_on_exit = args.get("terminate_on_exit").and_then(|v| v.as_bool()).unwrap_or(true);
                        self.handle_launch(id, forked, terminate_on_exit);
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "Command launched");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: command_id");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            TerminalCommandMcpTools::Terminate => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let command_id = args.get("command_id").and_then(|v| v.as_str());
                match command_id {
                    Some(id) => {
                        self.handle_terminate(id);
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "Command terminated");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: command_id");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            TerminalCommandMcpTools::Restart => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let command_id = args.get("command_id").and_then(|v| v.as_str());
                match command_id {
                    Some(id) => {
                        let forked = args.get("forked").and_then(|v| v.as_bool()).unwrap_or(false);
                        let terminate_on_exit = args.get("terminate_on_exit").and_then(|v| v.as_bool()).unwrap_or(true);
                        self.handle_restart(id, forked, terminate_on_exit);
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "Command restarted");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: command_id");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
        }
    }
}
