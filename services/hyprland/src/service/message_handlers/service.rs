use crate::service::HyprlandService;
use smearor_hyprland_model::ExecDispatchMessage;
use smearor_hyprland_model::HyprlandStateRequestMessage;
use smearor_hyprland_model::HyprlandSystemDispatchMessage;
use smearor_hyprland_model::HyprlandToggleDispatchMessage;
use smearor_hyprland_model::HyprlandWindowDispatchMessage;
use smearor_hyprland_model::HyprlandWorkspaceDispatchMessage;
use smearor_hyprland_model::KillActiveWindowDispatchMessage;
use smearor_hyprland_model::KillCommandMessage;
use smearor_hyprland_model::MoveFocusDispatchMessage;
use smearor_hyprland_model::NotifyCommandMessage;
use smearor_hyprland_model::OutputCreateCommandMessage;
use smearor_hyprland_model::OutputRemoveCommandMessage;
use smearor_hyprland_model::PluginLoadCommandMessage;
use smearor_hyprland_model::PluginUnloadCommandMessage;
use smearor_hyprland_model::ReloadCommandMessage;
use smearor_hyprland_model::SetCursorCommandMessage;
use smearor_hyprland_model::SetErrorCommandMessage;
use smearor_hyprland_model::SetPropCommandMessage;
use smearor_hyprland_model::SwitchXkbLayoutCommandMessage;
use smearor_hyprland_model::ToggleFullscreenDispatchMessage;
use smearor_hyprland_model::WorkspaceDispatchMessage;
use smearor_model_compositor::CreateWorkspaceMessage;
use smearor_model_compositor::SwitchWorkspaceMessage;
use smearor_model_compositor::WorkspaceSnapshotRequestMessage;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::ServicePlugin;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use tracing::debug;
use tracing::trace;

impl MessageBroadcaster for HyprlandService {}

impl PluginMetaGetter for HyprlandService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for HyprlandService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl ServicePlugin for HyprlandService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            trace!("Hyprland service received message: topic={}, type_id={}", envelope.topic.to_string(), envelope.type_id);
            match envelope.type_id {
                id if id == FfiEnvelopePayload::<HyprlandWindowDispatchMessage>::TYPE_ID => {
                    debug!("HyprlandWindowDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<HyprlandWindowDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<HyprlandWorkspaceDispatchMessage>::TYPE_ID => {
                    debug!("HyprlandWorkspaceDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<HyprlandWorkspaceDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<HyprlandToggleDispatchMessage>::TYPE_ID => {
                    debug!("HyprlandToggleDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<HyprlandToggleDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<HyprlandSystemDispatchMessage>::TYPE_ID => {
                    debug!("HyprlandSystemDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<HyprlandSystemDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<WorkspaceDispatchMessage>::TYPE_ID => {
                    debug!("WorkspaceDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<WorkspaceDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<ExecDispatchMessage>::TYPE_ID => {
                    debug!("ExecDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<ExecDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<KillActiveWindowDispatchMessage>::TYPE_ID => {
                    debug!("KillActiveWindowDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<KillActiveWindowDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<MoveFocusDispatchMessage>::TYPE_ID => {
                    debug!("MoveFocusDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<MoveFocusDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<ToggleFullscreenDispatchMessage>::TYPE_ID => {
                    debug!("ToggleFullscreenDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<ToggleFullscreenDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<SwitchWorkspaceMessage>::TYPE_ID => {
                    debug!("SwitchWorkspaceMessage");
                    MessageHandler::<FfiEnvelopePayload<SwitchWorkspaceMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<CreateWorkspaceMessage>::TYPE_ID => {
                    debug!("CreateWorkspaceMessage");
                    MessageHandler::<FfiEnvelopePayload<CreateWorkspaceMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<WorkspaceSnapshotRequestMessage>::TYPE_ID => {
                    debug!("WorkspaceSnapshotRequestMessage");
                    MessageHandler::<FfiEnvelopePayload<WorkspaceSnapshotRequestMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<KillCommandMessage>::TYPE_ID => {
                    debug!("KillCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<KillCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<NotifyCommandMessage>::TYPE_ID => {
                    debug!("NotifyCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<NotifyCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<OutputCreateCommandMessage>::TYPE_ID => {
                    debug!("OutputCreateCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<OutputCreateCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<OutputRemoveCommandMessage>::TYPE_ID => {
                    debug!("OutputRemoveCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<OutputRemoveCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<PluginLoadCommandMessage>::TYPE_ID => {
                    debug!("PluginLoadCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<PluginLoadCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<PluginUnloadCommandMessage>::TYPE_ID => {
                    debug!("PluginUnloadCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<PluginUnloadCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<ReloadCommandMessage>::TYPE_ID => {
                    debug!("ReloadCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<ReloadCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<SetCursorCommandMessage>::TYPE_ID => {
                    debug!("SetCursorCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<SetCursorCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<SetErrorCommandMessage>::TYPE_ID => {
                    debug!("SetErrorCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<SetErrorCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<SetPropCommandMessage>::TYPE_ID => {
                    debug!("SetPropCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<SetPropCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<SwitchXkbLayoutCommandMessage>::TYPE_ID => {
                    debug!("SwitchXkbLayoutCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<SwitchXkbLayoutCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<HyprlandStateRequestMessage>::TYPE_ID => {
                    debug!("HyprlandStateRequestMessage");
                    MessageHandler::<FfiEnvelopePayload<HyprlandStateRequestMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID => {
                    debug!("InvokeToolMessage");
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID => {
                    debug!("InvokeResourceMessage");
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                }
                _ => {
                    trace!("Hyprland service: unhandled message type for topic {}", envelope.topic.to_string());
                }
            }
        }
    }
}
