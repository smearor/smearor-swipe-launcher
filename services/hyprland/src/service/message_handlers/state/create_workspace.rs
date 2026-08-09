use crate::service::HyprlandService;
use crate::service::command::HyprlandCommand;
use smearor_model_compositor::CreateWorkspaceMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use tracing::trace;

impl MessageHandler<FfiEnvelopePayload<CreateWorkspaceMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<CreateWorkspaceMessage>, _sender_id: &str) {
        trace!(
            "Hyprland service: queueing workspace creation relative_to={}, position={:?}",
            message.0.relative_to, message.0.position
        );
        let _ = self.command_sender.send(HyprlandCommand::CreateWorkspace(message.0));
    }
}
