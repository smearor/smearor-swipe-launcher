use crate::service::AppLauncherService;
use smearor_app_launcher_model::AppLauncherMcpResources;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl McpResourceHandler<AppLauncherMcpResources> for AppLauncherService {
    fn get_response(&self, request: &ResourceRequest<AppLauncherMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        match request.resource {
            AppLauncherMcpResources::RunningApps => {
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
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            AppLauncherMcpResources::AvailableApps => {
                let (offset, limit) = parse_pagination_params(request.resource.as_ref());
                let apps = self.available_apps_snapshot();
                let total = apps.len();
                let offset = offset.min(total);
                let end = if limit == 0 { total } else { (offset + limit).min(total) };
                let page = &apps[offset..end];
                let json = serde_json::json!({
                    "available_apps": page.iter().map(|(path, name)| {
                        serde_json::json!({
                            "desktop_file": path,
                            "name": name,
                        })
                    }).collect::<Vec<_>>(),
                    "pagination": {
                        "offset": offset,
                        "limit": if limit == 0 { total } else { limit },
                        "total": total,
                        "returned": page.len(),
                        "has_more": end < total,
                    },
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for AppLauncherService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        self.handle_invoke_resource_message(message, sender_id);
    }
}

/// Parses `offset` and `limit` query parameters from a URI.
///
/// Returns `(0, 0)` when no pagination parameters are present, which
/// signals "return everything". Supports formats like:
/// - `app_launcher://available_apps?offset=20&limit=10`
/// - `app_launcher://available_apps?limit=50`
fn parse_pagination_params(uri: &str) -> (usize, usize) {
    let Some(query) = uri.split('?').nth(1) else {
        return (0, 0);
    };
    let mut offset = 0;
    let mut limit = 0;
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        match key {
            "offset" => {
                if let Ok(parsed) = value.parse::<usize>() {
                    offset = parsed;
                }
            }
            "limit" => {
                if let Ok(parsed) = value.parse::<usize>() {
                    limit = parsed;
                }
            }
            _ => {}
        }
    }
    (offset, limit)
}
