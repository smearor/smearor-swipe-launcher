use crate::service::HyprlandService;
use crate::service::command::HyprlandCommand;
use smearor_hyprland_model::HyprlandToggleDispatchMessage;
use smearor_hyprland_model::ToggleDispatchKind;
use smearor_hyprland_model::ToggleDispatchOps;
use smearor_hyprland_model::ToggleFullscreenDispatchMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use stabby::option::Option as StabbyOption;

impl MessageHandler<FfiEnvelopePayload<HyprlandToggleDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<HyprlandToggleDispatchMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::ToggleDispatch(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<ToggleFullscreenDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<ToggleFullscreenDispatchMessage>, _sender_id: &str) {
        let dispatch_message = HyprlandToggleDispatchMessage {
            kind: ToggleDispatchKind::ToggleFullscreen,
            ops: ToggleDispatchOps {
                toggle_fullscreen: StabbyOption::Some(message.0.into()),
                ..ToggleDispatchOps::default()
            },
        };
        let _ = self.command_sender.send(HyprlandCommand::ToggleDispatch(dispatch_message));
    }
}
