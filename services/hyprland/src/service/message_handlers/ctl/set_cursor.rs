use crate::service::HyprlandService;
use crate::service::command::HyprlandCommand;
use smearor_hyprland_model::SetCursorCommandMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl MessageHandler<FfiEnvelopePayload<SetCursorCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<SetCursorCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlSetCursor(message.0));
    }
}
