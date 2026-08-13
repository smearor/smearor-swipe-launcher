use crate::service::ThemeService;
use serde::Serialize;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_theme_model::Theme;
use smearor_theme_model::ThemeMcpResources;
use smearor_theme_model::ThemeMode;
use tracing::debug;
use tracing::error;

/// MCP resource response for `theme://status`.
#[derive(Serialize)]
struct ThemeStatusResource {
    current_theme: Option<String>,
    selected_theme_index: u32,
    effective_mode: ThemeMode,
}

/// MCP resource response for `theme://themes`.
#[derive(Serialize)]
struct ThemeListResource {
    themes: Vec<ThemeInfoResource>,
}

/// Single theme entry in the `theme://themes` resource response.
#[derive(Serialize)]
struct ThemeInfoResource {
    name: String,
    description: String,
    preview_icon: String,
    mode: ThemeMode,
    has_wallpaper: bool,
}

impl From<&Theme> for ThemeInfoResource {
    fn from(t: &Theme) -> Self {
        Self {
            name: t.name.clone(),
            description: t.description.clone(),
            preview_icon: t.preview_icon.clone(),
            mode: t.mode,
            has_wallpaper: t.wallpaper_theme.is_some(),
        }
    }
}

impl McpResourceHandler<ThemeMcpResources> for ThemeService {
    fn get_response(&self, request: &ResourceRequest<ThemeMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        match request.resource {
            ThemeMcpResources::Status => {
                let state_guard = self.state.read();
                let (selected_index, current_theme, effective_mode) = match state_guard {
                    Ok(s) => (s.selected_theme_index, s.current_theme.clone(), s.effective_mode),
                    Err(e) => {
                        error!("Theme service: state lock poisoned: {e}");
                        (0, None, ThemeMode::Dark)
                    }
                };
                let resource = ThemeStatusResource {
                    current_theme: current_theme.map(|c| c.to_string()),
                    selected_theme_index: selected_index as u32,
                    effective_mode,
                };
                let json = serde_json::to_string(&resource).unwrap_or_default();
                InvokeResourceResponse::success(correlation_id, &json)
            }
            ThemeMcpResources::Themes => {
                let state_guard = self.state.read();
                let themes: Vec<ThemeInfoResource> = match state_guard {
                    Ok(s) => s.themes.iter().map(ThemeInfoResource::from).collect(),
                    Err(e) => {
                        error!("Theme service: state lock poisoned: {e}");
                        Vec::new()
                    }
                };
                let resource = ThemeListResource { themes };
                let json = serde_json::to_string(&resource).unwrap_or_default();
                InvokeResourceResponse::success(correlation_id, &json)
            }
        }
    }

    fn send_resource_response(&self, response: InvokeResourceResponse, sender_id: &str) {
        self.send_response(response, sender_id);
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for ThemeService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        debug!("theme: InvokeResourceMessage uri={}", message.0.uri);
        self.handle_invoke_resource_message(message, sender_id);
    }
}
