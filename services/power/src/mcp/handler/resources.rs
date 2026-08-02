use crate::service::PowerService;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_power_model::PowerMcpResources;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl McpResourceHandler<PowerMcpResources> for PowerService {
    fn get_response(&self, request: &ResourceRequest<PowerMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        let state = self.state_snapshot();
        match request.resource {
            PowerMcpResources::Capabilities => {
                let caps = &state.capabilities;
                let json = serde_json::json!({
                    "can_shutdown": caps.can_shutdown,
                    "can_reboot": caps.can_reboot,
                    "can_suspend": caps.can_suspend,
                    "can_hibernate": caps.can_hibernate,
                    "can_reboot_to_firmware": caps.can_reboot_to_firmware,
                    "can_lock": caps.can_lock,
                    "can_logout": caps.can_logout,
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            PowerMcpResources::Inhibitors => {
                let inhibitors: Vec<serde_json::Value> = state
                    .inhibitors
                    .iter()
                    .map(|inh| {
                        serde_json::json!({
                            "process_name": inh.process_name.to_string(),
                            "reason": inh.reason.to_string(),
                            "what": inh.what.to_string(),
                            "who": inh.who.to_string(),
                        })
                    })
                    .collect();
                let json = serde_json::Value::Array(inhibitors);
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            PowerMcpResources::ScheduledActions => {
                let json = match state.scheduled_action.as_ref() {
                    Some(sched) => serde_json::json!({
                        "action": format!("{:?}", sched.action),
                        "remaining_seconds": sched.remaining_seconds,
                        "total_delay_seconds": sched.total_delay_seconds,
                    }),
                    None => serde_json::json!(null),
                };
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for PowerService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        self.handle_invoke_resource_message(message, sender_id);
    }
}
