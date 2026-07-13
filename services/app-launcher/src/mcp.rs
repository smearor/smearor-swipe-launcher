use crate::service::AppLauncherService;
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

impl AppLauncherService {
    pub fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let running_apps_resource = RegisterResourceMessage::new(
            "app_launcher://running_apps",
            "Running Applications",
            "List of currently running tracked applications with their PIDs and termination policy.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(running_apps_resource);

        let available_apps_resource = RegisterResourceMessage::new(
            "app_launcher://available_apps",
            "Available Applications",
            "List of all available .desktop files found in standard application directories, sorted by name.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(available_apps_resource);

        let exec_tool = RegisterToolMessage::new(
            "app_launcher_exec",
            "Launch an application by desktop file path. The desktop file path must be the canonical path to a .desktop file.",
            r#"{ "type": "object", "properties": { "desktop_file": { "type": "string", "description": "Canonical path to the .desktop file" }, "forked": { "type": "boolean", "description": "Whether the process should be detached from the launcher (default: false)" }, "terminate_on_exit": { "type": "boolean", "description": "Whether to terminate the process when the launcher exits (default: true)" } }, "required": ["desktop_file"] }"#,
        );
        broadcaster.broadcast_message_to_topic(exec_tool);

        let terminate_tool = RegisterToolMessage::new(
            "app_launcher_terminate",
            "Terminate a running application by desktop file path.",
            r#"{ "type": "object", "properties": { "desktop_file": { "type": "string", "description": "Canonical path to the .desktop file" } }, "required": ["desktop_file"] }"#,
        );
        broadcaster.broadcast_message_to_topic(terminate_tool);
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for AppLauncherService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("AppLauncher Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();

        match tool_name.as_str() {
            "app_launcher_exec" => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let desktop_file = args.get("desktop_file").and_then(|v| v.as_str());
                match desktop_file {
                    Some(path) => {
                        let forked = args.get("forked").and_then(|v| v.as_bool()).unwrap_or(false);
                        let terminate_on_exit = args.get("terminate_on_exit").and_then(|v| v.as_bool()).unwrap_or(true);
                        self.handle_exec(path, None, forked, terminate_on_exit);
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "Application launched");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: desktop_file");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            "app_launcher_terminate" => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let desktop_file = args.get("desktop_file").and_then(|v| v.as_str());
                match desktop_file {
                    Some(path) => {
                        self.handle_terminate(path);
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "Application terminated");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: desktop_file");
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

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for AppLauncherService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, _sender_id: &str) {
        let uri = message.0.uri.to_string();
        debug!("AppLauncher Service: InvokeResourceMessage uri={}", uri);
        let broadcaster = self.get_broadcaster();

        let response = match uri.as_str() {
            "app_launcher://running_apps" => {
                let snapshot = self.running_apps_snapshot();
                let json = serde_json::json!({
                    "running_apps": snapshot.iter().map(|(desktop_file, pids, terminate_on_exit)| {
                        serde_json::json!({
                            "desktop_file": desktop_file,
                            "pids": pids,
                            "terminate_on_exit": terminate_on_exit,
                        })
                    }).collect::<Vec<_>>(),
                });
                InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
            }
            "app_launcher://available_apps" => {
                let apps = self.available_apps_snapshot();
                let json = serde_json::json!({
                    "available_apps": apps.iter().map(|(path, name)| {
                        serde_json::json!({
                            "desktop_file": path,
                            "name": name,
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
