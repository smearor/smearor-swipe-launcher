use crate::config::AudioServiceConfig;
use crate::pulse::run_pulse_async;
use crate::pulse_command::PulseCommand;
use glib::MainContext;
use smearor_audio_model::AudioCommandAction;
use smearor_audio_model::AudioCommandMessage;
use smearor_audio_model::AudioStatusMessage;
use smearor_doa_model::DoaStatusMessage;
use smearor_doa_model::TOPIC_STATUS as TOPIC_DOA_STATUS;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::ServicePlugin;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;
use smearor_voice_assistant_model::AssistantState;
use smearor_voice_assistant_model::AssistantStatusMessage;
use smearor_voice_assistant_model::TOPIC_STATUS as TOPIC_VA_STATUS;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use tracing::debug;
use tracing::error;
use tracing::trace;

pub struct AudioService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    #[allow(unused)]
    pub config: AudioServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<PulseCommand>,
    pub last_status: Arc<Mutex<Option<AudioStatusMessage>>>,
    /// Previous `speech_detected` value from DoA status, used for edge detection.
    pub previous_speech_detected: Arc<Mutex<bool>>,
    /// Whether audio is currently ducked (volume reduced due to speech detection).
    pub is_ducked: Arc<Mutex<bool>>,
    /// Cancellation token for the ducking grace period restore timer.
    pub duck_grace_cancel: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    /// Timestamp of the first rising edge of `speech_detected`, used for
    /// `min_speech_duration_ms` false-trigger mitigation.
    pub vad_onset_timestamp: Arc<Mutex<Option<Instant>>>,
    /// Whether the Voice Assistant is currently speaking (TTS active).
    pub tts_active: Arc<Mutex<bool>>,
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
                    destroy_payload: Some(default_destroy_payload),
                    clone_payload: Some(default_clone_payload::<AudioStatusMessage>),
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
            previous_speech_detected: Arc::new(Mutex::new(false)),
            is_ducked: Arc::new(Mutex::new(false)),
            duck_grace_cancel: Arc::new(Mutex::new(None)),
            vad_onset_timestamp: Arc::new(Mutex::new(None)),
            tts_active: Arc::new(Mutex::new(false)),
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
            AudioCommandAction::Refresh => self.handle_refresh_status(),
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<DoaStatusMessage>> for AudioService {
    fn handle_message(&self, message: FfiEnvelopePayload<DoaStatusMessage>, _sender_id: &str) {
        if !self.config.ducking_enabled {
            return;
        }

        // TTS-aware ducking suppression: skip ducking during TTS when configured.
        if !self.config.duck_during_tts {
            if let Ok(tts) = self.tts_active.lock() {
                if *tts {
                    // Reset edge detection state during TTS.
                    if let Ok(mut prev) = self.previous_speech_detected.lock() {
                        *prev = false;
                    }
                    if let Ok(mut onset) = self.vad_onset_timestamp.lock() {
                        *onset = None;
                    }
                    return;
                }
            }
        }

        let speech_detected = message.0.speech_detected;
        let previous_speech = self.previous_speech_detected.lock().map(|p| *p).unwrap_or(false);

        if speech_detected && !previous_speech {
            // Rising edge: speech started — record onset timestamp.
            debug!("Audio Service: DoA VAD rising edge detected");
            if let Ok(mut onset) = self.vad_onset_timestamp.lock() {
                *onset = Some(Instant::now());
            }

            // Cancel any pending grace period restore.
            if let Ok(mut cancel) = self.duck_grace_cancel.lock() {
                if let Some(sender) = cancel.take() {
                    let _ = sender.send(());
                }
            }
        } else if speech_detected && previous_speech {
            // Continuous speech: check min_speech_duration_ms for ducking activation.
            let should_duck = {
                let onset_opt = self.vad_onset_timestamp.lock().map(|o| *o).unwrap_or(None);
                if let Some(onset) = onset_opt {
                    let elapsed = onset.elapsed().as_millis() as u64;
                    elapsed >= self.config.min_speech_duration_ms
                } else {
                    false
                }
            };

            if should_duck {
                // Clear onset timestamp so we only duck once per rising edge.
                if let Ok(mut onset) = self.vad_onset_timestamp.lock() {
                    *onset = None;
                }

                let already_ducked = self.is_ducked.lock().map(|d| *d).unwrap_or(false);
                if !already_ducked {
                    debug!("Audio Service: DoA VAD ducking volume to {}", self.config.ducking_volume);
                    let _ = self.command_sender.send(PulseCommand::DuckVolume(self.config.ducking_volume));
                    if let Ok(mut ducked) = self.is_ducked.lock() {
                        *ducked = true;
                    }
                }
            }
        } else if !speech_detected && previous_speech {
            // Falling edge: speech stopped — schedule grace period restore.
            debug!("Audio Service: DoA VAD falling edge, scheduling volume restore in {} ms", self.config.ducking_grace_period_ms);
            if let Ok(mut onset) = self.vad_onset_timestamp.lock() {
                *onset = None;
            }

            // Cancel any existing grace period timer.
            if let Ok(mut cancel) = self.duck_grace_cancel.lock() {
                if let Some(sender) = cancel.take() {
                    let _ = sender.send(());
                }
            }

            let grace_period_ms = self.config.ducking_grace_period_ms;
            let fade_ramp_ms = self.config.fade_ramp_ms;
            let command_sender_clone = self.command_sender.clone();
            let is_ducked_clone = self.is_ducked.clone();
            let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();

            if let Ok(mut cancel) = self.duck_grace_cancel.lock() {
                *cancel = Some(cancel_tx);
            }

            glib::MainContext::default().spawn_local(async move {
                let timer = tokio::time::sleep(std::time::Duration::from_millis(grace_period_ms));
                tokio::pin!(timer);

                tokio::select! {
                    _ = timer => {
                        debug!("Audio Service: ducking grace period expired, restoring volume with fade ramp {} ms", fade_ramp_ms);
                        let _ = command_sender_clone.send(PulseCommand::FadeRestoreVolume { target: 1.0, ramp_ms: fade_ramp_ms });
                        if let Ok(mut ducked) = is_ducked_clone.lock() {
                            *ducked = false;
                        }
                    }
                    _ = cancel_rx => {
                        debug!("Audio Service: ducking grace period cancelled (speech resumed)");
                    }
                }
            });
        }

        // Update previous speech detected state.
        if let Ok(mut prev) = self.previous_speech_detected.lock() {
            *prev = speech_detected;
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<AssistantStatusMessage>> for AudioService {
    fn handle_message(&self, message: FfiEnvelopePayload<AssistantStatusMessage>, _sender_id: &str) {
        let is_speaking = message.0.current_state == AssistantState::Speaking;
        if let Ok(mut tts) = self.tts_active.lock() {
            *tts = is_speaking;
        }
        if is_speaking {
            debug!("Audio Service: Voice Assistant TTS started, tts_active = true");
        } else {
            debug!("Audio Service: Voice Assistant TTS ended, tts_active = false");
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

impl ServicePlugin for AudioService {
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
                } else if topic == TOPIC_DOA_STATUS && envelope.type_id == FfiEnvelopePayload::<DoaStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<DoaStatusMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_VA_STATUS && envelope.type_id == FfiEnvelopePayload::<AssistantStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<AssistantStatusMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}
