use crate::service::NotificationService;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_notifications_model::NotificationDndResponse;
use smearor_notifications_model::NotificationHistoryEntry;
use smearor_notifications_model::NotificationHistoryResponse;
use smearor_notifications_model::NotificationMcpResources;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl McpResourceHandler<NotificationMcpResources> for NotificationService {
    fn get_response(&self, request: &ResourceRequest<NotificationMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        let Some(status) = self.status_snapshot() else {
            return InvokeResourceResponse::error(correlation_id, "Notification status not yet available");
        };

        match request.resource {
            NotificationMcpResources::History => {
                let notifications: Vec<NotificationHistoryEntry> = status
                    .notifications
                    .iter()
                    .map(|n| NotificationHistoryEntry {
                        id: n.id,
                        app_name: n.app_name.to_string(),
                        summary: n.summary.to_string(),
                        body: n.body.to_string(),
                        urgency: format!("{:?}", n.urgency),
                        timestamp: n.timestamp,
                    })
                    .collect();
                let response = NotificationHistoryResponse {
                    do_not_disturb: status.do_not_disturb,
                    unread_count: status.unread_count,
                    notifications,
                };
                let json = serde_json::to_string(&response).unwrap_or_default();
                InvokeResourceResponse::success(correlation_id, &json)
            }
            NotificationMcpResources::Dnd => {
                let response = NotificationDndResponse {
                    do_not_disturb: status.do_not_disturb,
                };
                let json = serde_json::to_string(&response).unwrap_or_default();
                InvokeResourceResponse::success(correlation_id, &json)
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for NotificationService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        self.handle_invoke_resource_message(message, sender_id);
    }
}
