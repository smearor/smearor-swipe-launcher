use crate::service::AppLauncherService;
use smearor_app_launcher_model::AppLauncherMcpTools;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for AppLauncherService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("AppLauncher Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match AppLauncherMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &message.0.correlation_id)));
                return;
            }
        };
        match tool {
            AppLauncherMcpTools::Exec => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let desktop_file = args.get("desktop_file").and_then(|v| v.as_str());
                match desktop_file {
                    Some(path) => {
                        let forked = args.get("forked").and_then(|v| v.as_bool()).unwrap_or(false);
                        let terminate_on_exit = args.get("terminate_on_exit").and_then(|v| v.as_bool()).unwrap_or(true);
                        let response = match self.handle_exec(path, None, forked, terminate_on_exit) {
                            Ok(()) => InvokeToolResponse::success(&message.0.correlation_id, "Application launched"),
                            Err(error) => InvokeToolResponse::error(&message.0.correlation_id, &error),
                        };
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: desktop_file");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            AppLauncherMcpTools::SearchApps => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                if query.is_empty() {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: query");
                    broadcaster.broadcast_message_to_topic(response);
                } else {
                    let apps = self.available_apps_snapshot();
                    let query_lower = query.to_lowercase();
                    let matches: Vec<_> = apps.iter().filter(|(_, name)| name.to_lowercase().contains(&query_lower)).collect();
                    let json = serde_json::json!({
                        "available_apps": matches.iter().map(|(path, name)| {
                            serde_json::json!({
                                "desktop_file": path,
                                "name": name,
                            })
                        }).collect::<Vec<_>>(),
                        "count": matches.len(),
                    });
                    let response = InvokeToolResponse::success(&message.0.correlation_id, &json.to_string());
                    broadcaster.broadcast_message_to_topic(response);
                }
            }
            AppLauncherMcpTools::Terminate => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let desktop_file = args.get("desktop_file").and_then(|v| v.as_str());
                match desktop_file {
                    Some(path) => {
                        debug!("AppLauncher Service: handle_terminate for {path}");
                        self.handle_terminate(path);
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "Application terminated");
                        debug!("AppLauncher Service: sending InvokeToolResponse for correlation_id={}", message.0.correlation_id);
                        broadcaster.broadcast_message_to_topic(response);
                        debug!("AppLauncher Service: InvokeToolResponse sent");
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: desktop_file");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
        }
    }
}
