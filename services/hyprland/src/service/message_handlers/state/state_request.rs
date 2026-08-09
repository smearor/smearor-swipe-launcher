use crate::service::HyprlandService;
use crate::service::command::HyprlandCommand;
use smearor_hyprland_model::HyprlandStateRequestMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl MessageHandler<FfiEnvelopePayload<HyprlandStateRequestMessage>> for HyprlandService {
    fn handle_message(&self, _message: FfiEnvelopePayload<HyprlandStateRequestMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::StateRequest);
    }
}
