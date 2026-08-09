use crate::service::HyprlandService;
use crate::service::command::HyprlandCommand;
use smearor_hyprland_model::HyprlandSystemDispatchMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl MessageHandler<FfiEnvelopePayload<HyprlandSystemDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<HyprlandSystemDispatchMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::SystemDispatch(message.0));
    }
}
