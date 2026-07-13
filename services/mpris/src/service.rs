use crate::config::MprisServiceConfig;
use crate::dbus::run_mpris_async;
use crate::mpris_command::MprisCommand;
use glib::MainContext;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_mpris_model::MprisCommandAction;
use smearor_mpris_model::MprisCommandMessage;
use smearor_mpris_model::MprisStatusMessage;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::Service;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::error;
use tracing::trace;

pub struct MprisService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    #[allow(unused)]
    pub config: MprisServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<MprisCommand>,
    pub last_status: Arc<Mutex<Option<MprisStatusMessage>>>,
}

impl MprisService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        smearor_mpris_model::register_json_converters(core_context);

        let mpris_config: MprisServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel::<MprisCommand>();
        let (status_sender, mut status_receiver) = tokio::sync::mpsc::unbounded_channel::<MprisStatusMessage>();
        let last_status = Arc::new(Mutex::new(None::<MprisStatusMessage>));
        let meta = PluginMeta::try_from(&config)?;

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    error!("MPRIS Service: failed to create tokio runtime: {e}");
                    return;
                }
            };
            rt.block_on(async move {
                run_mpris_async(command_receiver, status_sender).await;
            });
        });

        let meta_clone = meta.clone();
        let core_context_clone = core_context.clone();
        let last_status_clone = last_status.clone();
        MainContext::default().spawn_local(async move {
            while let Some(status) = status_receiver.recv().await {
                if let Ok(mut last) = last_status_clone.lock() {
                    *last = Some(status.clone());
                }
                let payload_ptr = Box::into_raw(Box::new(status)) as *mut core::ffi::c_void;
                let envelope = FfiEnvelope {
                    sender_id: stabby::string::String::from(meta_clone.id.clone()),
                    target_instance_id: stabby::string::String::from("*"),
                    topic: stabby::string::String::from(MprisStatusMessage::topic()),
                    type_id: MprisStatusMessage::TYPE_ID,
                    payload: payload_ptr,
                    destroy_payload: Some(crate::dbus::destroy_mpris_status),
                    clone_payload: Some(crate::dbus::clone_mpris_status),
                };
                if let Some(ctx) = &core_context_clone {
                    ctx.send_message(envelope);
                }
            }
        });

        let service = MprisService {
            meta,
            core_context,
            config: mpris_config,
            command_sender,
            last_status: last_status.clone(),
        };
        service.register_mcp_capabilities();
        Ok(service)
    }

    pub(crate) fn handle_play(&self) {
        let _ = self.command_sender.send(MprisCommand::Play);
    }
    pub(crate) fn handle_pause(&self) {
        let _ = self.command_sender.send(MprisCommand::Pause);
    }
    pub(crate) fn handle_toggle_play_pause(&self) {
        let _ = self.command_sender.send(MprisCommand::TogglePlayPause);
    }
    pub(crate) fn handle_stop(&self) {
        let _ = self.command_sender.send(MprisCommand::Stop);
    }
    pub(crate) fn handle_next_track(&self) {
        let _ = self.command_sender.send(MprisCommand::NextTrack);
    }
    pub(crate) fn handle_previous_track(&self) {
        let _ = self.command_sender.send(MprisCommand::PreviousTrack);
    }
    pub(crate) fn handle_seek(&self, offset: i64) {
        let _ = self.command_sender.send(MprisCommand::Seek(offset));
    }
    pub(crate) fn handle_set_position(&self, position: i64) {
        let _ = self.command_sender.send(MprisCommand::SetPosition(position));
    }
    pub(crate) fn handle_cycle_loop(&self) {
        let _ = self.command_sender.send(MprisCommand::CycleLoop);
    }
    pub(crate) fn handle_toggle_shuffle(&self) {
        let _ = self.command_sender.send(MprisCommand::ToggleShuffle);
    }
    pub(crate) fn handle_next_player(&self) {
        let _ = self.command_sender.send(MprisCommand::NextPlayer);
    }
    pub(crate) fn handle_previous_player(&self) {
        let _ = self.command_sender.send(MprisCommand::PreviousPlayer);
    }
    pub(crate) fn handle_raise(&self) {
        let _ = self.command_sender.send(MprisCommand::Raise);
    }
    pub(crate) fn handle_quit(&self) {
        let _ = self.command_sender.send(MprisCommand::Quit);
    }

    pub(crate) fn status_snapshot(&self) -> Option<MprisStatusMessage> {
        self.last_status.lock().ok().and_then(|s| s.clone())
    }
}

impl MessageHandler<FfiEnvelopePayload<MprisCommandMessage>> for MprisService {
    fn handle_message(&self, message: FfiEnvelopePayload<MprisCommandMessage>, _sender_id: &str) {
        trace!("MPRIS Service: received command {:?}", message.action);
        match message.action {
            MprisCommandAction::Play => self.handle_play(),
            MprisCommandAction::Pause => self.handle_pause(),
            MprisCommandAction::TogglePlayPause => self.handle_toggle_play_pause(),
            MprisCommandAction::Stop => self.handle_stop(),
            MprisCommandAction::NextTrack => self.handle_next_track(),
            MprisCommandAction::PreviousTrack => self.handle_previous_track(),
            MprisCommandAction::Seek => {
                if let Some(o) = message.seek_offset.as_ref().copied() {
                    self.handle_seek(o);
                }
            }
            MprisCommandAction::SetPosition => {
                if let Some(p) = message.position.as_ref().copied() {
                    self.handle_set_position(p);
                }
            }
            MprisCommandAction::CycleLoop => self.handle_cycle_loop(),
            MprisCommandAction::ToggleShuffle => self.handle_toggle_shuffle(),
            MprisCommandAction::NextPlayer => self.handle_next_player(),
            MprisCommandAction::PreviousPlayer => self.handle_previous_player(),
            MprisCommandAction::Raise => self.handle_raise(),
            MprisCommandAction::Quit => self.handle_quit(),
        }
    }
}

impl MessageBroadcaster for MprisService {}
impl MessageTopicBroadcaster<MprisStatusMessage> for MprisService {}
impl PluginMetaGetter for MprisService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}
impl AsRef<Option<FfiCoreContext>> for MprisService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Service for MprisService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                let topic = envelope.topic.to_string();
                if envelope.type_id == FfiEnvelopePayload::<MprisCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<MprisCommandMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_TOOL && envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_RESOURCE && envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}
