use crate::service::HyprlandService;
use smearor_hyprland_model::ActiveWindowEntry;
use smearor_hyprland_model::HyprlandMcpResources;
use smearor_hyprland_model::HyprlandStateResponse;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl McpResourceHandler<HyprlandMcpResources> for HyprlandService {
    fn get_response(&self, request: &ResourceRequest<HyprlandMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        let Some(state) = self.status_snapshot() else {
            return InvokeResourceResponse::error(correlation_id, "Hyprland state not yet available");
        };

        match request.resource {
            HyprlandMcpResources::State => {
                let active_window = state.active_window.as_ref().map(|w| ActiveWindowEntry {
                    class: w.window_class.to_string(),
                    title: w.window_title.to_string(),
                    workspace_id: w.workspace_id,
                });
                let response = HyprlandStateResponse {
                    active_window,
                    is_fullscreen: state.is_fullscreen,
                    keyboard_layout: state.keyboard_layout.as_ref().map(|l| l.to_string()).unwrap_or_else(|| "unknown".to_string()),
                    submap: state.sub_map.to_string(),
                };
                let json = serde_json::to_string(&response).unwrap_or_default();
                InvokeResourceResponse::success(correlation_id, &json)
            }
            HyprlandMcpResources::ActiveWindow => match state.active_window.as_ref() {
                Some(w) => {
                    let entry = ActiveWindowEntry {
                        class: w.window_class.to_string(),
                        title: w.window_title.to_string(),
                        workspace_id: w.workspace_id,
                    };
                    let json = serde_json::to_string(&entry).unwrap_or_default();
                    InvokeResourceResponse::success(correlation_id, &json)
                }
                None => InvokeResourceResponse::success(correlation_id, "null"),
            },
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        self.handle_invoke_resource_message(message, sender_id);
    }
}
