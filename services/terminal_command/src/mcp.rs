use crate::service::TerminalCommandService;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use tracing::debug;

impl TerminalCommandService {
    /// Registers all MCP resources and tools exposed by the terminal command service.
    pub fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let running_resource = RegisterResourceMessage::new(
            "terminal_command://running",
            "Running Terminal Commands",
            "List of currently running tracked terminal commands with their PIDs and termination policy.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(running_resource);

        let configured_resource = RegisterResourceMessage::new(
            "terminal_command://configured",
            "Configured Terminal Commands",
            "List of all configured terminal commands from services.toml with their command, args, and options.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(configured_resource);

        let launch_tool = RegisterToolMessage::new(
            "terminal_command_launch",
            "Launch a configured terminal command by command_id.",
            r#"{ "type": "object", "properties": { "command_id": { "type": "string", "description": "The configured command identifier" }, "forked": { "type": "boolean", "description": "Whether the process should be detached from the launcher (default: false)" }, "terminate_on_exit": { "type": "boolean", "description": "Whether to terminate the process when the launcher exits (default: true)" } }, "required": ["command_id"] }"#,
        );
        broadcaster.broadcast_message_to_topic(launch_tool);

        let terminate_tool = RegisterToolMessage::new(
            "terminal_command_terminate",
            "Terminate a running terminal command by command_id.",
            r#"{ "type": "object", "properties": { "command_id": { "type": "string", "description": "The configured command identifier" } }, "required": ["command_id"] }"#,
        );
        broadcaster.broadcast_message_to_topic(terminate_tool);

        let restart_tool = RegisterToolMessage::new(
            "terminal_command_restart",
            "Restart a terminal command by command_id (terminate then launch).",
            r#"{ "type": "object", "properties": { "command_id": { "type": "string", "description": "The configured command identifier" }, "forked": { "type": "boolean", "description": "Whether the process should be detached from the launcher (default: false)" }, "terminate_on_exit": { "type": "boolean", "description": "Whether to terminate the process when the launcher exits (default: true)" } }, "required": ["command_id"] }"#,
        );
        broadcaster.broadcast_message_to_topic(restart_tool);
    }

    /// Returns a snapshot of all running tracked commands.
    pub fn running_commands_snapshot(&self) -> Vec<(String, Vec<u32>, bool)> {
        self.tracked_processes
            .iter()
            .map(|entry| {
                let command_id = entry.key().clone();
                let pids = entry.value().iter().map(|tp| tp.pid).collect::<Vec<_>>();
                let terminate_on_exit = entry.value().first().map(|tp| tp.terminate_on_exit).unwrap_or(false);
                (command_id, pids, terminate_on_exit)
            })
            .collect()
    }

    /// Returns a snapshot of all configured commands.
    pub fn configured_commands_snapshot(&self) -> Vec<(String, String, Vec<String>, bool)> {
        self.config
            .commands
            .iter()
            .map(|(command_id, definition)| (command_id.clone(), definition.command.clone(), definition.args.clone(), definition.restart_on_exit))
            .collect()
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for TerminalCommandService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("TerminalCommand Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();

        match tool_name.as_str() {
            "terminal_command_launch" => {
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
            "terminal_command_terminate" => {
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
            "terminal_command_restart" => {
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
            _ => {
                let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Unknown tool: {tool_name}"));
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for TerminalCommandService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, _sender_id: &str) {
        let uri = message.0.uri.to_string();
        debug!("TerminalCommand Service: InvokeResourceMessage uri={}", uri);
        let broadcaster = self.get_broadcaster();

        let response = match uri.as_str() {
            "terminal_command://running" => {
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
                InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
            }
            "terminal_command://configured" => {
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
                InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
            }
            _ => InvokeResourceResponse::error(&message.0.correlation_id, &format!("Unknown resource: {uri}")),
        };
        broadcaster.broadcast_message_to_topic(response);
    }
}
