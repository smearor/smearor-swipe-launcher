use crate::service::WallpaperService;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_wallpaper_model::WallpaperMcpResources;
use tracing::debug;
use tracing::error;

impl McpResourceHandler<WallpaperMcpResources> for WallpaperService {
    fn get_response(&self, request: &ResourceRequest<WallpaperMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        match request.resource {
            WallpaperMcpResources::Status => {
                let state_guard = self.state.read();
                let (selected_index, current_theme, current_processes) = match state_guard {
                    Ok(s) => (s.selected_theme_index, s.current_theme.clone(), s.current_processes.clone()),
                    Err(e) => {
                        error!("Wallpaper service: state lock poisoned: {e}");
                        (0, None, Vec::new())
                    }
                };
                let config_guard = self.config.read();
                let themes: Vec<serde_json::Value> = match config_guard {
                    Ok(config) => config
                        .load_themes()
                        .iter()
                        .map(|t| {
                            serde_json::json!({
                                "name": t.name,
                                "description": t.description,
                                "preview_image_path": t.preview_image_path,
                                "preview_icon": t.preview_icon,
                                "wallpaper_type": format!("{:?}", t.wallpaper_type),
                            })
                        })
                        .collect(),
                    Err(e) => {
                        error!("Wallpaper service: config lock poisoned: {e}");
                        Vec::new()
                    }
                };
                let current_processes_json: Vec<serde_json::Value> = current_processes
                    .iter()
                    .map(|p| serde_json::json!({ "monitor": p.monitor.to_string(), "process_id": p.process_id }))
                    .collect();
                let json = serde_json::json!({
                    "current_theme": current_theme,
                    "current_processes": current_processes_json,
                    "selected_theme_index": selected_index,
                    "themes": themes,
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            WallpaperMcpResources::Themes => {
                let config_guard = self.config.read();
                let themes: Vec<serde_json::Value> = match config_guard {
                    Ok(config) => config
                        .load_themes()
                        .iter()
                        .map(|t| {
                            serde_json::json!({
                                "name": t.name,
                                "description": t.description,
                                "preview_image_path": t.preview_image_path,
                                "preview_icon": t.preview_icon,
                                "wallpaper_type": format!("{:?}", t.wallpaper_type),
                            })
                        })
                        .collect(),
                    Err(e) => {
                        error!("Wallpaper service: config lock poisoned: {e}");
                        Vec::new()
                    }
                };
                let json = serde_json::json!({ "themes": themes });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
        }
    }

    fn send_resource_response(&self, response: InvokeResourceResponse, sender_id: &str) {
        self.send_response(response, sender_id);
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for WallpaperService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        debug!("wallpaper: InvokeResourceMessage uri={}", message.0.uri);
        self.handle_invoke_resource_message(message, sender_id);
    }
}
