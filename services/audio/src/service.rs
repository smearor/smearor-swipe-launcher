use crate::config::AudioServiceConfig;
use crate::pulse::clone_audio_status;
use crate::pulse::destroy_audio_status;
use crate::pulse::run_pulse_async;
use crate::pulse_command::PulseCommand;
use glib::MainContext;
use smearor_audio_model::AudioCommandAction;
use smearor_audio_model::AudioCommandMessage;
use smearor_audio_model::AudioStatusMessage;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
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

pub struct AudioService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    #[allow(unused)]
    pub config: AudioServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<PulseCommand>,
    pub last_status: Arc<Mutex<Option<AudioStatusMessage>>>,
}

impl AudioService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        smearor_audio_model::register_json_converters(core_context);

        let audio_config: AudioServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;

        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel::<PulseCommand>();
        let (status_sender, mut status_receiver) = tokio::sync::mpsc::unbounded_channel::<AudioStatusMessage>();
        let last_status = Arc::new(Mutex::new(None::<AudioStatusMessage>));

        let meta = PluginMeta::try_from(&config)?;

        let audio_config_inner = audio_config.clone();
        let command_sender_clone = command_sender.clone();
        let last_status_for_pulse = last_status.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    error!("Audio Service: failed to create tokio runtime: {e}");
                    return;
                }
            };
            rt.block_on(async move {
                run_pulse_async(command_receiver, command_sender_clone, status_sender, audio_config_inner, last_status_for_pulse).await;
            });
        });

        let meta_clone = meta.clone();
        let core_context_clone = core_context.clone();
        MainContext::default().spawn_local(async move {
            while let Some(status) = status_receiver.recv().await {
                let payload_ptr = Box::into_raw(Box::new(status)) as *mut core::ffi::c_void;
                let envelope = FfiEnvelope {
                    sender_id: stabby::string::String::from(meta_clone.id.clone()),
                    target_instance_id: stabby::string::String::from("*"),
                    topic: stabby::string::String::from(AudioStatusMessage::topic()),
                    type_id: AudioStatusMessage::TYPE_ID,
                    payload: payload_ptr,
                    destroy_payload: Some(destroy_audio_status),
                    clone_payload: Some(clone_audio_status),
                };
                if let Some(ctx) = &core_context_clone {
                    ctx.send_message(envelope);
                }
            }
        });

        let service = AudioService {
            meta: meta.clone(),
            core_context,
            config: audio_config,
            command_sender: command_sender.clone(),
            last_status: last_status.clone(),
        };
        service.register_mcp_capabilities();
        Ok(service)
    }

    pub(crate) fn handle_volume_up(&self) {
        let _ = self.command_sender.send(PulseCommand::VolumeUp);
    }

    pub(crate) fn handle_volume_down(&self) {
        let _ = self.command_sender.send(PulseCommand::VolumeDown);
    }

    pub(crate) fn handle_set_volume(&self, volume: f32) {
        let _ = self.command_sender.send(PulseCommand::SetVolume(volume));
    }

    pub(crate) fn handle_toggle_mute(&self) {
        let _ = self.command_sender.send(PulseCommand::ToggleMute);
    }

    pub(crate) fn handle_mute(&self) {
        let _ = self.command_sender.send(PulseCommand::Mute);
    }

    pub(crate) fn handle_unmute(&self) {
        let _ = self.command_sender.send(PulseCommand::Unmute);
    }

    pub(crate) fn handle_next_device(&self) {
        let _ = self.command_sender.send(PulseCommand::NextDevice);
    }

    pub(crate) fn handle_previous_device(&self) {
        let _ = self.command_sender.send(PulseCommand::PreviousDevice);
    }

    pub(crate) fn handle_refresh_status(&self) {
        let _ = self.command_sender.send(PulseCommand::RefreshStatus);
    }

    pub(crate) fn status_snapshot(&self) -> Option<AudioStatusMessage> {
        self.last_status.lock().ok().and_then(|s| s.clone())
    }
}

impl MessageHandler<FfiEnvelopePayload<AudioCommandMessage>> for AudioService {
    fn handle_message(&self, message: FfiEnvelopePayload<AudioCommandMessage>, _sender_id: &str) {
        trace!("Audio Service: received command {:?}", message.action);
        match message.action {
            AudioCommandAction::VolumeUp => self.handle_volume_up(),
            AudioCommandAction::VolumeDown => self.handle_volume_down(),
            AudioCommandAction::SetVolume => {
                let volume_opt: Option<f32> = message.volume.clone().into();
                if let Some(volume) = volume_opt {
                    self.handle_set_volume(volume);
                }
            }
            AudioCommandAction::ToggleMute => self.handle_toggle_mute(),
            AudioCommandAction::Mute => self.handle_mute(),
            AudioCommandAction::Unmute => self.handle_unmute(),
            AudioCommandAction::NextDevice => self.handle_next_device(),
            AudioCommandAction::PreviousDevice => self.handle_previous_device(),
        }
    }
}

impl MessageBroadcaster for AudioService {}

impl MessageTopicBroadcaster<AudioStatusMessage> for AudioService {}

impl PluginMetaGetter for AudioService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for AudioService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Service for AudioService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                let topic = envelope.topic.to_string();
                if envelope.type_id == FfiEnvelopePayload::<AudioCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<AudioCommandMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_TOOL && envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_RESOURCE && envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}
