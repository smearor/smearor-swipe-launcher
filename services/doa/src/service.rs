use crate::config::DoaServiceConfig;
use crate::state::DoaSharedState;
use crate::usb::DoaReading;
use crate::usb::UsbControl;
use crate::usb::usb_reader_loop;
use smearor_doa_model::DoaCommandAction;
use smearor_doa_model::DoaCommandMessage;
use smearor_doa_model::DoaDirection;
use smearor_doa_model::DoaStatusMessage;
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
use smearor_swipe_launcher_plugin_api::box_payload;
use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::debug;
use tracing::error;
use tracing::warn;

pub struct DoaService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: DoaServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<DoaCommandMessage>,
    pub command_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<DoaCommandMessage>>,
    pub shared_state: Arc<Mutex<DoaSharedState>>,
}

impl DoaService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        smearor_doa_model::register_json_converters(core_context);

        let doa_config: DoaServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;

        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel::<DoaCommandMessage>();
        let meta = PluginMeta::try_from(&config)?;
        let shared_state = Arc::new(Mutex::new(DoaSharedState::default()));

        let service = DoaService {
            meta,
            core_context,
            config: doa_config,
            command_sender,
            command_receiver: Some(command_receiver),
            shared_state,
        };
        Ok(service)
    }

    pub(crate) fn state_snapshot(&self) -> DoaSharedState {
        self.shared_state
            .lock()
            .map(|s| DoaSharedState {
                connected: s.connected,
                angle: s.angle,
                calibrated_angle: s.calibrated_angle,
                rotation_offset: s.rotation_offset,
                speech_detected: s.speech_detected,
                vendor_id: s.vendor_id,
                product_id: s.product_id,
                last_updated: s.last_updated.clone(),
                paused: s.paused,
            })
            .unwrap_or_default()
    }
}

impl MessageHandler<FfiEnvelopePayload<DoaCommandMessage>> for DoaService {
    fn handle_message(&self, message: FfiEnvelopePayload<DoaCommandMessage>, _sender_id: &str) {
        debug!("DoA Service: received command {:?}", message.action);
        match message.action {
            DoaCommandAction::Reconnect => {
                let _ = self.command_sender.send(DoaCommandMessage {
                    action: DoaCommandAction::Reconnect,
                    value: 0,
                });
            }
            DoaCommandAction::Pause => {
                let _ = self.command_sender.send(DoaCommandMessage {
                    action: DoaCommandAction::Pause,
                    value: 0,
                });
            }
            DoaCommandAction::Resume => {
                let _ = self.command_sender.send(DoaCommandMessage {
                    action: DoaCommandAction::Resume,
                    value: 0,
                });
            }
            DoaCommandAction::SetPollInterval => {
                let _ = self.command_sender.send(DoaCommandMessage {
                    action: DoaCommandAction::SetPollInterval,
                    value: message.value.max(50),
                });
            }
        }
    }
}

impl MessageBroadcaster for DoaService {}

impl MessageTopicBroadcaster<DoaStatusMessage> for DoaService {}

impl PluginMetaGetter for DoaService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for DoaService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl ServicePlugin for DoaService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                let topic = envelope.topic.to_string();
                if envelope.type_id == FfiEnvelopePayload::<DoaCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<DoaCommandMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_TOOL && envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_RESOURCE && envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }

    fn start(&mut self) {
        if let Some(ctx) = &self.core_context {
            let meta = self.meta.clone();
            let core_context = *ctx;
            let command_receiver = self.command_receiver.take();
            let config = self.config.clone();
            let shared_state = self.shared_state.clone();
            self.register_mcp_capabilities();
            std::thread::spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(rt) => rt,
                    Err(e) => {
                        error!("DoA Service: failed to create tokio runtime: {e}");
                        return;
                    }
                };
                let local_set = tokio::task::LocalSet::new();
                local_set.block_on(&rt, async move {
                    if let Some(receiver) = command_receiver {
                        run_doa_async(meta, Some(core_context), receiver, config, shared_state).await;
                    }
                });
            });
        }
    }
}

impl Drop for DoaService {
    fn drop(&mut self) {
        debug!("DoA Service: dropping, command_sender will be released");
    }
}

async fn run_doa_async(
    meta: PluginMeta,
    core_context: Option<FfiCoreContext>,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<DoaCommandMessage>,
    config: DoaServiceConfig,
    shared_state: Arc<Mutex<DoaSharedState>>,
) {
    let (reading_sender, mut reading_receiver) = tokio::sync::mpsc::unbounded_channel::<DoaReading>();
    let (usb_control_sender, usb_control_receiver) = tokio::sync::mpsc::unbounded_channel::<UsbControl>();

    let rotation_offset = config.rotation_offset;
    let ceiling_mode = config.ceiling_mode;
    let config_for_usb = config.clone();
    std::thread::spawn(move || {
        usb_reader_loop(config_for_usb, reading_sender, usb_control_receiver);
    });

    let mut last_angle: Option<u16> = None;
    let mut last_speech_detected: Option<bool> = None;

    loop {
        tokio::select! {
            command = command_receiver.recv() => {
                match command {
                    Some(cmd) => handle_command(&cmd, &shared_state, &usb_control_sender, &meta, &core_context),
                    None => {
                        debug!("DoA async loop: command channel closed, shutting down");
                        break;
                    }
                }
            }
            Some(reading) = reading_receiver.recv() => {
                if let Some(status) = process_reading(&reading, &shared_state, rotation_offset, ceiling_mode, &mut last_angle, &mut last_speech_detected) {
                    broadcast_status(&meta, &core_context, status);
                }
            }
        }
    }
}

fn handle_command(
    command: &DoaCommandMessage,
    shared_state: &Arc<Mutex<DoaSharedState>>,
    usb_control: &tokio::sync::mpsc::UnboundedSender<UsbControl>,
    meta: &PluginMeta,
    core_context: &Option<FfiCoreContext>,
) {
    match command.action {
        DoaCommandAction::Reconnect => {
            let _ = usb_control.send(UsbControl::Reconnect);
        }
        DoaCommandAction::Pause => {
            {
                let mut state = shared_state.lock().unwrap();
                state.paused = true;
            }
            let _ = usb_control.send(UsbControl::Pause);
            broadcast_status_from_state(shared_state, meta, core_context);
        }
        DoaCommandAction::Resume => {
            {
                let mut state = shared_state.lock().unwrap();
                state.paused = false;
            }
            let _ = usb_control.send(UsbControl::Resume);
            broadcast_status_from_state(shared_state, meta, core_context);
        }
        DoaCommandAction::SetPollInterval => {
            let interval = command.value.max(50);
            let _ = usb_control.send(UsbControl::SetInterval(interval));
        }
    }
}

fn broadcast_status_from_state(shared_state: &Arc<Mutex<DoaSharedState>>, meta: &PluginMeta, core_context: &Option<FfiCoreContext>) {
    let state = shared_state.lock().map(|s| s.clone()).unwrap_or_default();
    let status = DoaStatusMessage {
        connected: state.connected,
        angle: state.angle,
        calibrated_angle: state.calibrated_angle,
        direction: DoaDirection::from_angle(state.calibrated_angle),
        speech_detected: state.speech_detected,
        vendor_id: state.vendor_id,
        product_id: state.product_id,
        last_updated: stabby::string::String::from(state.last_updated),
        paused: state.paused,
    };
    broadcast_status(meta, core_context, status);
}

fn broadcast_status(meta: &PluginMeta, core_context: &Option<FfiCoreContext>, status: DoaStatusMessage) {
    if let Some(ctx) = core_context {
        let payload_ptr = box_payload(status);
        let envelope = FfiEnvelope::builder()
            .sender_id(meta.id.clone())
            .target_instance_id("*")
            .topic(DoaStatusMessage::topic())
            .type_id(DoaStatusMessage::TYPE_ID)
            .payload(payload_ptr)
            .destroy_payload(Some(default_destroy_payload))
            .clone_payload(Some(default_clone_payload::<DoaStatusMessage>))
            .build();
        ctx.send_message(envelope);
    } else {
        warn!("DoA Service: no core context available, cannot broadcast status");
    }
}

fn current_timestamp() -> String {
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    format!("{}", duration.as_secs())
}

/// Computes the calibrated angle from the raw angle, rotation offset, and ceiling mode.
///
/// When `ceiling_mode` is true, the raw angle is mirrored (360 - angle) before
/// applying the rotation offset, compensating for a ceiling-mounted installation.
pub fn compute_calibrated_angle(angle: u16, rotation_offset: i16, ceiling_mode: bool) -> u16 {
    let raw_angle = if ceiling_mode { (360 - angle as i16).rem_euclid(360) } else { angle as i16 };
    (raw_angle + rotation_offset).rem_euclid(360) as u16
}

/// Processes a `DoaReading` and updates the shared state.
///
/// Returns `Some(DoaStatusMessage)` if the status changed (angle or speech detection),
/// or `None` if the reading is identical to the previous one.
pub fn process_reading(
    reading: &DoaReading,
    shared_state: &Arc<Mutex<DoaSharedState>>,
    rotation_offset: i16,
    ceiling_mode: bool,
    last_angle: &mut Option<u16>,
    last_speech_detected: &mut Option<bool>,
) -> Option<DoaStatusMessage> {
    match reading {
        DoaReading::Reading {
            angle,
            speech_detected,
            vendor_id,
            product_id,
        } => {
            let calibrated_angle = compute_calibrated_angle(*angle, rotation_offset, ceiling_mode);
            let direction = DoaDirection::from_angle(calibrated_angle);
            let timestamp = current_timestamp();
            {
                let mut state = shared_state.lock().unwrap();
                state.connected = true;
                state.angle = *angle;
                state.calibrated_angle = calibrated_angle;
                state.rotation_offset = rotation_offset;
                state.speech_detected = *speech_detected;
                state.vendor_id = *vendor_id;
                state.product_id = *product_id;
                state.last_updated = timestamp.clone();
            }

            let changed = *last_angle != Some(*angle) || *last_speech_detected != Some(*speech_detected);
            if changed {
                *last_angle = Some(*angle);
                *last_speech_detected = Some(*speech_detected);
                let paused = shared_state.lock().map(|s| s.paused).unwrap_or(false);
                Some(DoaStatusMessage {
                    connected: true,
                    angle: *angle,
                    calibrated_angle,
                    direction,
                    speech_detected: *speech_detected,
                    vendor_id: *vendor_id,
                    product_id: *product_id,
                    last_updated: stabby::string::String::from(timestamp),
                    paused,
                })
            } else {
                None
            }
        }
        DoaReading::Disconnected => {
            *last_angle = None;
            *last_speech_detected = None;
            {
                let mut state = shared_state.lock().unwrap();
                state.connected = false;
                state.last_updated = current_timestamp();
            }
            let paused = shared_state.lock().map(|s| s.paused).unwrap_or(false);
            Some(DoaStatusMessage {
                connected: false,
                paused,
                ..Default::default()
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usb::DoaReading;
    use smearor_doa_model::DoaCommandAction;

    fn test_meta() -> PluginMeta {
        PluginMeta::new("test".to_string(), "test".to_string(), None)
    }

    #[test]
    fn test_compute_calibrated_angle_no_offset_no_ceiling() {
        assert_eq!(compute_calibrated_angle(0, 0, false), 0);
        assert_eq!(compute_calibrated_angle(90, 0, false), 90);
        assert_eq!(compute_calibrated_angle(180, 0, false), 180);
        assert_eq!(compute_calibrated_angle(270, 0, false), 270);
        assert_eq!(compute_calibrated_angle(359, 0, false), 359);
    }

    #[test]
    fn test_compute_calibrated_angle_with_positive_offset() {
        assert_eq!(compute_calibrated_angle(0, 90, false), 90);
        assert_eq!(compute_calibrated_angle(270, 90, false), 0);
        assert_eq!(compute_calibrated_angle(180, 90, false), 270);
    }

    #[test]
    fn test_compute_calibrated_angle_with_negative_offset() {
        assert_eq!(compute_calibrated_angle(0, -90, false), 270);
        assert_eq!(compute_calibrated_angle(90, -90, false), 0);
        assert_eq!(compute_calibrated_angle(0, -360, false), 0);
    }

    #[test]
    fn test_compute_calibrated_angle_ceiling_mode() {
        assert_eq!(compute_calibrated_angle(0, 0, true), 0);
        assert_eq!(compute_calibrated_angle(90, 0, true), 270);
        assert_eq!(compute_calibrated_angle(180, 0, true), 180);
        assert_eq!(compute_calibrated_angle(270, 0, true), 90);
    }

    #[test]
    fn test_compute_calibrated_angle_ceiling_mode_with_offset() {
        assert_eq!(compute_calibrated_angle(90, 45, true), 315);
        assert_eq!(compute_calibrated_angle(0, 180, true), 180);
    }

    #[test]
    fn test_compute_calibrated_angle_wraps_360() {
        assert_eq!(compute_calibrated_angle(350, 20, false), 10);
        assert_eq!(compute_calibrated_angle(10, -20, false), 350);
    }

    #[test]
    fn test_process_reading_disconnected() {
        let state = Arc::new(Mutex::new(DoaSharedState::default()));
        let mut last_angle = Some(90);
        let mut last_speech = Some(true);
        let result = process_reading(&DoaReading::Disconnected, &state, 0, false, &mut last_angle, &mut last_speech);
        let status = result.unwrap();
        assert!(!status.connected);
        assert_eq!(last_angle, None);
        assert_eq!(last_speech, None);
        assert!(!state.lock().unwrap().connected);
    }

    #[test]
    fn test_process_reading_first_reading_emits_status() {
        let state = Arc::new(Mutex::new(DoaSharedState::default()));
        let mut last_angle = None;
        let mut last_speech = None;
        let result = process_reading(
            &DoaReading::Reading {
                angle: 90,
                speech_detected: true,
                vendor_id: 0x2886,
                product_id: 0x0021,
            },
            &state,
            0,
            false,
            &mut last_angle,
            &mut last_speech,
        );
        let status = result.unwrap();
        assert!(status.connected);
        assert_eq!(status.angle, 90);
        assert_eq!(status.calibrated_angle, 90);
        assert!(status.speech_detected);
        assert_eq!(status.vendor_id, 0x2886);
        assert_eq!(last_angle, Some(90));
        assert_eq!(last_speech, Some(true));
    }

    #[test]
    fn test_process_reading_unchanged_does_not_emit() {
        let state = Arc::new(Mutex::new(DoaSharedState::default()));
        let mut last_angle = Some(90);
        let mut last_speech = Some(false);
        let result = process_reading(
            &DoaReading::Reading {
                angle: 90,
                speech_detected: false,
                vendor_id: 0x2886,
                product_id: 0x0021,
            },
            &state,
            0,
            false,
            &mut last_angle,
            &mut last_speech,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_process_reading_angle_changed_emits() {
        let state = Arc::new(Mutex::new(DoaSharedState::default()));
        let mut last_angle = Some(45);
        let mut last_speech = Some(false);
        let result = process_reading(
            &DoaReading::Reading {
                angle: 90,
                speech_detected: false,
                vendor_id: 0x2886,
                product_id: 0x0021,
            },
            &state,
            0,
            false,
            &mut last_angle,
            &mut last_speech,
        );
        let status = result.unwrap();
        assert_eq!(status.angle, 90);
        assert_eq!(last_angle, Some(90));
    }

    #[test]
    fn test_process_reading_speech_changed_emits() {
        let state = Arc::new(Mutex::new(DoaSharedState::default()));
        let mut last_angle = Some(90);
        let mut last_speech = Some(false);
        let result = process_reading(
            &DoaReading::Reading {
                angle: 90,
                speech_detected: true,
                vendor_id: 0x2886,
                product_id: 0x0021,
            },
            &state,
            0,
            false,
            &mut last_angle,
            &mut last_speech,
        );
        let status = result.unwrap();
        assert!(status.speech_detected);
        assert_eq!(last_speech, Some(true));
    }

    #[test]
    fn test_process_reading_with_offset() {
        let state = Arc::new(Mutex::new(DoaSharedState::default()));
        let mut last_angle = None;
        let mut last_speech = None;
        let result = process_reading(
            &DoaReading::Reading {
                angle: 0,
                speech_detected: false,
                vendor_id: 0x2886,
                product_id: 0x0021,
            },
            &state,
            90,
            false,
            &mut last_angle,
            &mut last_speech,
        );
        let status = result.unwrap();
        assert_eq!(status.angle, 0);
        assert_eq!(status.calibrated_angle, 90);
        assert_eq!(status.direction, DoaDirection::East);
    }

    #[test]
    fn test_process_reading_with_ceiling_mode() {
        let state = Arc::new(Mutex::new(DoaSharedState::default()));
        let mut last_angle = None;
        let mut last_speech = None;
        let result = process_reading(
            &DoaReading::Reading {
                angle: 90,
                speech_detected: false,
                vendor_id: 0x2886,
                product_id: 0x0021,
            },
            &state,
            0,
            true,
            &mut last_angle,
            &mut last_speech,
        );
        let status = result.unwrap();
        assert_eq!(status.angle, 90);
        assert_eq!(status.calibrated_angle, 270);
        assert_eq!(status.direction, DoaDirection::West);
    }

    #[test]
    fn test_process_reading_updates_shared_state() {
        let state = Arc::new(Mutex::new(DoaSharedState::default()));
        let mut last_angle = None;
        let mut last_speech = None;
        let _ = process_reading(
            &DoaReading::Reading {
                angle: 180,
                speech_detected: true,
                vendor_id: 0x2886,
                product_id: 0x0021,
            },
            &state,
            0,
            false,
            &mut last_angle,
            &mut last_speech,
        );
        let s = state.lock().unwrap();
        assert!(s.connected);
        assert_eq!(s.angle, 180);
        assert_eq!(s.calibrated_angle, 180);
        assert!(s.speech_detected);
        assert_eq!(s.vendor_id, 0x2886);
        assert_eq!(s.product_id, 0x0021);
        assert!(!s.last_updated.is_empty());
    }

    #[test]
    fn test_handle_command_reconnect() {
        let state = Arc::new(Mutex::new(DoaSharedState::default()));
        let (usb_tx, mut usb_rx) = tokio::sync::mpsc::unbounded_channel::<UsbControl>();
        let meta = test_meta();
        handle_command(
            &DoaCommandMessage {
                action: DoaCommandAction::Reconnect,
                value: 0,
            },
            &state,
            &usb_tx,
            &meta,
            &None,
        );
        assert!(matches!(usb_rx.try_recv(), Ok(UsbControl::Reconnect)));
    }

    #[test]
    fn test_handle_command_pause() {
        let state = Arc::new(Mutex::new(DoaSharedState::default()));
        let (usb_tx, mut usb_rx) = tokio::sync::mpsc::unbounded_channel::<UsbControl>();
        let meta = test_meta();
        handle_command(
            &DoaCommandMessage {
                action: DoaCommandAction::Pause,
                value: 0,
            },
            &state,
            &usb_tx,
            &meta,
            &None,
        );
        assert!(state.lock().unwrap().paused);
        assert!(matches!(usb_rx.try_recv(), Ok(UsbControl::Pause)));
    }

    #[test]
    fn test_handle_command_resume() {
        let state = Arc::new(Mutex::new(DoaSharedState {
            paused: true,
            ..Default::default()
        }));
        let (usb_tx, mut usb_rx) = tokio::sync::mpsc::unbounded_channel::<UsbControl>();
        let meta = test_meta();
        handle_command(
            &DoaCommandMessage {
                action: DoaCommandAction::Resume,
                value: 0,
            },
            &state,
            &usb_tx,
            &meta,
            &None,
        );
        assert!(!state.lock().unwrap().paused);
        assert!(matches!(usb_rx.try_recv(), Ok(UsbControl::Resume)));
    }

    #[test]
    fn test_handle_command_set_poll_interval() {
        let state = Arc::new(Mutex::new(DoaSharedState::default()));
        let (usb_tx, mut usb_rx) = tokio::sync::mpsc::unbounded_channel::<UsbControl>();
        let meta = test_meta();
        handle_command(
            &DoaCommandMessage {
                action: DoaCommandAction::SetPollInterval,
                value: 200,
            },
            &state,
            &usb_tx,
            &meta,
            &None,
        );
        assert!(matches!(usb_rx.try_recv(), Ok(UsbControl::SetInterval(200))));
    }

    #[test]
    fn test_handle_command_set_poll_interval_clamped_to_min() {
        let state = Arc::new(Mutex::new(DoaSharedState::default()));
        let (usb_tx, mut usb_rx) = tokio::sync::mpsc::unbounded_channel::<UsbControl>();
        let meta = test_meta();
        handle_command(
            &DoaCommandMessage {
                action: DoaCommandAction::SetPollInterval,
                value: 10,
            },
            &state,
            &usb_tx,
            &meta,
            &None,
        );
        assert!(matches!(usb_rx.try_recv(), Ok(UsbControl::SetInterval(50))));
    }
}
