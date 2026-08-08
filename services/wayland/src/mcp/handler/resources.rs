use crate::service::WaylandWorkspaceService;
use smearor_model_compositor::CompositorMcpResources;
use smearor_model_compositor::WorkspaceEntry;
use smearor_model_compositor::WorkspacesResponse;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl McpResourceHandler<CompositorMcpResources> for WaylandWorkspaceService {
    fn get_response(&self, request: &ResourceRequest<CompositorMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        let Some(snapshot) = self.status_snapshot() else {
            return InvokeResourceResponse::error(correlation_id, "Workspace snapshot not yet available");
        };

        match request.resource {
            CompositorMcpResources::Workspaces => {
                let workspaces: Vec<WorkspaceEntry> = snapshot
                    .workspaces
                    .iter()
                    .map(|w| WorkspaceEntry {
                        id: w.workspace_id,
                        name: w.workspace_name.to_string(),
                        monitor_index: w.monitor_index,
                        is_active: w.is_active,
                    })
                    .collect();
                let response = WorkspacesResponse {
                    active_workspace_id: snapshot.active_workspace_id,
                    active_monitor_index: snapshot.active_monitor_index,
                    workspaces,
                };
                let json = serde_json::to_string(&response).unwrap_or_default();
                InvokeResourceResponse::success(correlation_id, &json)
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for WaylandWorkspaceService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        self.handle_invoke_resource_message(message, sender_id);
    }
}
