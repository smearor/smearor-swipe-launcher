use crate::service::HyprlandService;
use crate::service::command::HyprlandCommand;
use smearor_hyprland_model::OutputCreateCommandMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl MessageHandler<FfiEnvelopePayload<OutputCreateCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<OutputCreateCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlOutputCreate(message.0));
    }
}
