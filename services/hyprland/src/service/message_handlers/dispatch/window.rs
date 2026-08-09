use crate::service::HyprlandService;
use crate::service::command::HyprlandCommand;
use smearor_hyprland_model::ExecDispatchMessage;
use smearor_hyprland_model::HyprlandWindowDispatchMessage;
use smearor_hyprland_model::KillActiveWindowDispatchMessage;
use smearor_hyprland_model::MoveFocusDispatchMessage;
use smearor_hyprland_model::WindowDispatchKind;
use smearor_hyprland_model::WindowDispatchOps;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use stabby::option::Option as StabbyOption;

impl MessageHandler<FfiEnvelopePayload<HyprlandWindowDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<HyprlandWindowDispatchMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<ExecDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<ExecDispatchMessage>, _sender_id: &str) {
        let dispatch_message = HyprlandWindowDispatchMessage {
            kind: WindowDispatchKind::Exec,
            ops: WindowDispatchOps {
                exec: StabbyOption::Some(message.0.into()),
                ..WindowDispatchOps::default()
            },
        };
        let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(dispatch_message));
    }
}

impl MessageHandler<FfiEnvelopePayload<KillActiveWindowDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<KillActiveWindowDispatchMessage>, _sender_id: &str) {
        let dispatch_message = HyprlandWindowDispatchMessage {
            kind: WindowDispatchKind::KillActiveWindow,
            ops: WindowDispatchOps {
                kill_active_window: StabbyOption::Some(message.0.into()),
                ..WindowDispatchOps::default()
            },
        };
        let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(dispatch_message));
    }
}

impl MessageHandler<FfiEnvelopePayload<MoveFocusDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<MoveFocusDispatchMessage>, _sender_id: &str) {
        let dispatch_message = HyprlandWindowDispatchMessage {
            kind: WindowDispatchKind::MoveFocus,
            ops: WindowDispatchOps {
                move_focus: StabbyOption::Some(message.0.into()),
                ..WindowDispatchOps::default()
            },
        };
        let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(dispatch_message));
    }
}
