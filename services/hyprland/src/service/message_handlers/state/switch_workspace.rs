use crate::service::HyprlandService;
use crate::service::command::HyprlandCommand;
use smearor_model_compositor::SwitchWorkspaceMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use tracing::trace;

impl MessageHandler<FfiEnvelopePayload<SwitchWorkspaceMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<SwitchWorkspaceMessage>, _sender_id: &str) {
        trace!("Hyprland service: queueing workspace switch to {}", message.0.workspace_id);
        let _ = self.command_sender.send(HyprlandCommand::SwitchWorkspace(message.0));
    }
}
