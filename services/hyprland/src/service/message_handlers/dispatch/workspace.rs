use crate::service::HyprlandService;
use crate::service::command::HyprlandCommand;
use smearor_hyprland_model::HyprlandWorkspaceDispatchMessage;
use smearor_hyprland_model::WorkspaceDispatchKind;
use smearor_hyprland_model::WorkspaceDispatchMessage;
use smearor_hyprland_model::WorkspaceDispatchOps;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use stabby::option::Option as StabbyOption;
use tracing::trace;

impl MessageHandler<FfiEnvelopePayload<HyprlandWorkspaceDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<HyprlandWorkspaceDispatchMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::WorkspaceDispatch(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<WorkspaceDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<WorkspaceDispatchMessage>, _sender_id: &str) {
        trace!("Hyprland service: queueing workspace dispatch for {:?}", message.0.identifier);
        let dispatch_message = HyprlandWorkspaceDispatchMessage {
            kind: WorkspaceDispatchKind::Workspace,
            ops: WorkspaceDispatchOps {
                workspace: StabbyOption::Some(message.0.into()),
                ..WorkspaceDispatchOps::default()
            },
        };
        let _ = self.command_sender.send(HyprlandCommand::WorkspaceDispatch(dispatch_message));
    }
}
