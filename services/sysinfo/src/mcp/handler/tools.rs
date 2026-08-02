use crate::service::SysinfoCommandAction;
use crate::service::SysinfoService;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_sysinfo_model::SysinfoMcpTools;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for SysinfoService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, sender_id: &str) {
        debug!("sysinfo: InvokeToolMessage handler name={} sender_id={}", message.0.name, sender_id);
        let broadcaster = self.get_broadcaster();
        let tool = match SysinfoMcpTools::from_str(&message.0.name.to_string()) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &message.0.correlation_id)));
                return;
            }
        };
        match tool {
            SysinfoMcpTools::Refresh => {
                let _ = self.command_sender.send(SysinfoCommandAction::Refresh);
                let correlation_id = message.0.correlation_id.to_string();
                let response = InvokeToolResponse::success(&correlation_id, "Refresh triggered");
                debug!("sysinfo: sending InvokeToolResponse correlation_id={}", correlation_id);
                self.send_response(response, sender_id);
            }
        }
    }
}
