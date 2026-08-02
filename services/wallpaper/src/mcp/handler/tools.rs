use crate::command::WallpaperCommand;
use crate::service::WallpaperService;
use crate::service::parse_theme_from_json;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_wallpaper_model::WallpaperMcpTools;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for WallpaperService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        let arguments_str = message.0.arguments.to_string();
        debug!("wallpaper: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match WallpaperMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &correlation_id)));
                return;
            }
        };
        match tool {
            WallpaperMcpTools::SelectTheme => {
                let args: serde_json::Value = serde_json::from_str(arguments_str.as_str()).unwrap_or_default();
                let theme_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let _ = self.command_sender.send(WallpaperCommand::SelectTheme(theme_name.to_string()));
                let response = InvokeToolResponse::success(&correlation_id, &format!("Selected theme: {theme_name}"));
                self.send_response(response, sender_id);
            }
            WallpaperMcpTools::StartSelectedProcess => {
                let _ = self.command_sender.send(WallpaperCommand::StartSelected);
                let response = InvokeToolResponse::success(&correlation_id, "Start command sent");
                self.send_response(response, sender_id);
            }
            WallpaperMcpTools::StopCurrentProcess => {
                let _ = self.command_sender.send(WallpaperCommand::StopCurrent);
                let response = InvokeToolResponse::success(&correlation_id, "Stop command sent");
                self.send_response(response, sender_id);
            }
            WallpaperMcpTools::AddTheme => {
                let args: serde_json::Value = serde_json::from_str(arguments_str.as_str()).unwrap_or_default();
                match parse_theme_from_json(&args) {
                    Ok(theme) => {
                        let theme_name = theme.name.clone();
                        let _ = self.command_sender.send(WallpaperCommand::AddTheme(theme));
                        let response = InvokeToolResponse::success(&correlation_id, &format!("Added theme: {theme_name}"));
                        self.send_response(response, sender_id);
                    }
                    Err(e) => {
                        let response = InvokeToolResponse::error(&correlation_id, &format!("Failed to parse theme: {e}"));
                        self.send_response(response, sender_id);
                    }
                }
            }
            WallpaperMcpTools::RemoveTheme => {
                let args: serde_json::Value = serde_json::from_str(arguments_str.as_str()).unwrap_or_default();
                let theme_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if theme_name.is_empty() {
                    let response = InvokeToolResponse::error(&correlation_id, "Missing required field: name");
                    self.send_response(response, sender_id);
                } else {
                    let _ = self.command_sender.send(WallpaperCommand::RemoveTheme(theme_name.to_string()));
                    let response = InvokeToolResponse::success(&correlation_id, &format!("Removed theme: {theme_name}"));
                    self.send_response(response, sender_id);
                }
            }
        }
    }
}
