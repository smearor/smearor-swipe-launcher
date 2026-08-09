use crate::service::HyprlandService;
use crate::service::command::HyprlandCommand;
use smearor_hyprland_model::PluginUnloadCommandMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl MessageHandler<FfiEnvelopePayload<PluginUnloadCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<PluginUnloadCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlPluginUnload(message.0));
    }
}
