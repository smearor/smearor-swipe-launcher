use crate::service::HyprlandService;
use crate::service::command::HyprlandCommand;
use smearor_model_compositor::WorkspaceSnapshotRequestMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use tracing::trace;

impl MessageHandler<FfiEnvelopePayload<WorkspaceSnapshotRequestMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<WorkspaceSnapshotRequestMessage>, _sender_id: &str) {
        trace!("Hyprland service: queueing workspace snapshot request");
        let _ = self.command_sender.send(HyprlandCommand::SnapshotRequest(message.0));
    }
}
