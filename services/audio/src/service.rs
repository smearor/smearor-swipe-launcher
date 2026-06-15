use crate::config::AudioServiceConfig;
use abi_stable::std_types::RString;
use glib::ControlFlow;
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::Context;
use libpulse_binding::context::FlagSet;
use libpulse_binding::context::introspect::Introspector;
use libpulse_binding::context::introspect::ServerInfo;
use libpulse_binding::context::subscribe::Facility;
use libpulse_binding::mainloop::threaded::Mainloop;
use libpulse_binding::proplist::Proplist;
use libpulse_binding::volume::ChannelVolumes;
use libpulse_binding::volume::Volume;
use smearor_audio_model::AudioCommandAction;
use smearor_audio_model::AudioCommandMessage;
use smearor_audio_model::AudioStatusMessage;
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
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::mpsc::Sender;
use std::time::Duration;
use std::time::Instant;
use tracing::debug;
use tracing::error;
use tracing::info;

pub enum PulseCommand {
    VolumeUp,
    VolumeDown,
    SetVolume(f32),
    ToggleMute,
    Mute,
    Unmute,
    NextDevice,
    PreviousDevice,
    RefreshStatus,
}

/// Tracks the current PulseAudio sink state for command execution.
#[derive(Clone, Debug)]
struct PulseState {
    /// Name of the default sink.
    default_sink_name: Option<String>,
    /// Index of the default sink.
    default_sink_index: Option<u32>,
    /// Current volume ratio (0.0 - 1.5).
    volume: f32,
    /// Whether the default sink is muted.
    mute: bool,
    /// Number of channels on the default sink.
    channels: u8,
    /// Available output sinks: (index, name).
    sinks: Vec<(u32, String)>,
    /// Whether a device switch is in progress and pulse_state should not be overwritten by query_status.
    pending_switch: bool,
}

impl Default for PulseState {
    fn default() -> Self {
        Self {
            default_sink_name: None,
            default_sink_index: None,
            volume: 0.0,
            mute: false,
            channels: 2,
            sinks: Vec::new(),
            pending_switch: false,
        }
    }
}

pub struct AudioService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AudioServiceConfig,
    pub command_sender: Sender<PulseCommand>,
    pub status_receiver: Arc<Mutex<Receiver<AudioStatusMessage>>>,
}

impl AudioService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let audio_config: AudioServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;

        let (command_sender, command_receiver) = mpsc::channel::<PulseCommand>();
        let (status_sender, status_receiver) = mpsc::channel::<AudioStatusMessage>();

        let meta = PluginMeta::try_from(&config)?;

        let audio_config_inner = audio_config.clone();
        let command_sender_clone = command_sender.clone();
        std::thread::spawn(move || {
            run_pulse_thread(command_receiver, command_sender_clone, status_sender, audio_config_inner);
        });

        let service = AudioService {
            meta,
            core_context,
            config: audio_config,
            command_sender,
            status_receiver: Arc::new(Mutex::new(status_receiver)),
        };

        let meta_clone = service.meta.clone();
        let core_context_clone = service.core_context.clone();
        let status_receiver_clone = Arc::clone(&service.status_receiver);
        glib::timeout_add_local(Duration::from_millis(50), move || {
            while let Ok(status) = status_receiver_clone.lock().unwrap().try_recv() {
                if let Ok(payload) = serde_json::to_string(&status) {
                    let envelope = FfiEnvelope {
                        sender_id: RString::from(meta_clone.id.clone()),
                        topic: RString::from(AudioStatusMessage::topic()),
                        payload: RString::from(payload),
                    };
                    if let Some(ctx) = &core_context_clone {
                        ctx.send_message(envelope);
                    }
                }
            }
            ControlFlow::Continue
        });

        Ok(service)
    }

    fn handle_volume_up(&self) {
        let _ = self.command_sender.send(PulseCommand::VolumeUp);
    }

    fn handle_volume_down(&self) {
        let _ = self.command_sender.send(PulseCommand::VolumeDown);
    }

    fn handle_set_volume(&self, volume: f32) {
        let _ = self.command_sender.send(PulseCommand::SetVolume(volume));
    }

    fn handle_toggle_mute(&self) {
        let _ = self.command_sender.send(PulseCommand::ToggleMute);
    }

    fn handle_mute(&self) {
        let _ = self.command_sender.send(PulseCommand::Mute);
    }

    fn handle_unmute(&self) {
        let _ = self.command_sender.send(PulseCommand::Unmute);
    }

    fn handle_next_device(&self) {
        let _ = self.command_sender.send(PulseCommand::NextDevice);
    }

    fn handle_previous_device(&self) {
        let _ = self.command_sender.send(PulseCommand::PreviousDevice);
    }
}

impl MessageHandler<FfiEnvelopePayload<AudioCommandMessage>> for AudioService {
    fn handle_message(&self, message: FfiEnvelopePayload<AudioCommandMessage>, _sender_id: &str) {
        info!("Audio Service: received command {:?}", message.action);
        match message.action {
            AudioCommandAction::VolumeUp => self.handle_volume_up(),
            AudioCommandAction::VolumeDown => self.handle_volume_down(),
            AudioCommandAction::SetVolume => {
                if let Some(volume) = message.volume {
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

impl MessageBroadcaster<AudioStatusMessage> for AudioService {}

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

fn run_pulse_thread(
    command_receiver: Receiver<PulseCommand>,
    command_sender: Sender<PulseCommand>,
    status_sender: Sender<AudioStatusMessage>,
    _config: AudioServiceConfig,
) {
    let mut mainloop = match Mainloop::new() {
        Some(ml) => ml,
        None => {
            error!("Audio Service: Failed to create PulseAudio mainloop");
            return;
        }
    };

    let proplist = match Proplist::new() {
        Some(pl) => pl,
        None => {
            error!("Audio Service: Failed to create PulseAudio proplist");
            return;
        }
    };
    let mut context = match Context::new_with_proplist(&mainloop, "SmearorAudioService", &proplist) {
        Some(ctx) => ctx,
        None => {
            error!("Audio Service: Failed to create PulseAudio context");
            return;
        }
    };

    let mainloop_ptr: *mut Mainloop = &mut mainloop;
    let context_ptr: *mut Context = &mut context;
    let ready = Arc::new(AtomicBool::new(false));
    let ready_clone = ready.clone();

    context.set_state_callback(Some(Box::new(move || {
        let state = unsafe { (*context_ptr).get_state() };
        match state {
            libpulse_binding::context::State::Ready | libpulse_binding::context::State::Failed | libpulse_binding::context::State::Terminated => {
                ready_clone.store(true, Ordering::SeqCst);
                unsafe {
                    (*mainloop_ptr).signal(false);
                }
            }
            _ => {}
        }
    })));

    if let Err(err) = context.connect(None, FlagSet::NOFLAGS, None) {
        error!("Audio Service: Failed to connect to PulseAudio: {:?}", err);
        return;
    }

    if let Err(err) = mainloop.start() {
        error!("Audio Service: Failed to start mainloop: {:?}", err);
        return;
    }

    mainloop.lock();
    while !ready.load(Ordering::SeqCst) {
        mainloop.wait();
    }
    mainloop.unlock();

    let state = context.get_state();
    if state != libpulse_binding::context::State::Ready {
        error!("Audio Service: Failed to connect to PulseAudio, state: {:?}", state);
        context.disconnect();
        return;
    }

    context.set_state_callback(None);
    debug!("Audio Service: PulseAudio context ready");

    let pulse_state = Arc::new(Mutex::new(PulseState::default()));
    let last_status = Arc::new(Mutex::new(None::<AudioStatusMessage>));
    let last_refresh = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1)));

    let _ = context.subscribe(Facility::Sink.to_interest_mask(), |_| {});
    let command_sender_clone = command_sender.clone();
    context.set_subscribe_callback(Some(Box::new(move |facility, _operation, _index| {
        if facility == Some(Facility::Sink) {
            let now = Instant::now();
            let Ok(mut last) = last_refresh.lock() else {
                return;
            };
            if now.duration_since(*last) > Duration::from_millis(100) {
                *last = now;
                debug!("PulseAudio sink event detected, triggering status refresh");
                let _ = command_sender_clone.send(PulseCommand::RefreshStatus);
            }
        }
    })));

    // Trigger initial status broadcast so widgets get state immediately.
    let _ = command_sender.send(PulseCommand::RefreshStatus);

    let mut introspect = context.introspect();
    let mut last_refresh_time = Instant::now() - Duration::from_secs(1);
    let mut pending_refresh = false;

    loop {
        match command_receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(command) => match command {
                PulseCommand::VolumeUp => {
                    debug!("Command Receiver: Volume up command received");
                    if let Ok(s) = pulse_state.lock() {
                        if let Some(ref name) = s.default_sink_name {
                            let new_vol = (s.volume + 0.05).min(1.0);
                            let mut cv = ChannelVolumes::default();
                            cv.set(s.channels, Volume((Volume::NORMAL.0 as f32 * new_vol) as u32));
                            debug!("Command Receiver: set_sink_volume_by_name {name} to {new_vol}");
                            introspect.set_sink_volume_by_name(name, &cv, Some(Box::new(|_| {})));
                        }
                    }
                    if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                        pending_refresh = true;
                    }
                }
                PulseCommand::VolumeDown => {
                    debug!("Command Receiver: Volume down command received");
                    if let Ok(s) = pulse_state.lock() {
                        if let Some(ref name) = s.default_sink_name {
                            let new_vol = (s.volume - 0.05).max(0.0);
                            let mut cv = ChannelVolumes::default();
                            cv.set(s.channels, Volume((Volume::NORMAL.0 as f32 * new_vol) as u32));
                            debug!("Command Receiver: set_sink_volume_by_name {name} to {new_vol}");
                            introspect.set_sink_volume_by_name(name, &cv, Some(Box::new(|_| {})));
                        }
                    }
                    if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                        pending_refresh = true;
                    }
                }
                PulseCommand::SetVolume(volume) => {
                    debug!("Command Receiver: Set volume command received");
                    if let Ok(s) = pulse_state.lock() {
                        if let Some(ref name) = s.default_sink_name {
                            let new_vol = volume.clamp(0.0, 1.0);
                            let mut cv = ChannelVolumes::default();
                            cv.set(s.channels, Volume((Volume::NORMAL.0 as f32 * new_vol) as u32));
                            debug!("Command Receiver: set_sink_volume_by_name {name} to {new_vol}");
                            introspect.set_sink_volume_by_name(name, &cv, Some(Box::new(|_| {})));
                        }
                    }
                    if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                        pending_refresh = true;
                    }
                }
                PulseCommand::ToggleMute => {
                    debug!("Command Receiver: toggle mute command received");
                    if let Ok(s) = pulse_state.lock() {
                        if let Some(ref name) = s.default_sink_name {
                            debug!("Command Receiver: set_sink_mute_by_name {name} to {}", !s.mute);
                            introspect.set_sink_mute_by_name(name, !s.mute, Some(Box::new(|_| {})));
                        }
                    }
                    if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                        pending_refresh = true;
                    }
                }
                PulseCommand::Mute => {
                    debug!("Command Receiver: mute command received");
                    if let Ok(s) = pulse_state.lock() {
                        if let Some(ref name) = s.default_sink_name {
                            debug!("Command Receiver: set_sink_mute_by_name {name} to {}", true);
                            introspect.set_sink_mute_by_name(name, true, Some(Box::new(|_| {})));
                        }
                    }
                    if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                        pending_refresh = true;
                    }
                }
                PulseCommand::Unmute => {
                    debug!("Command Receiver: unmute command received");
                    if let Ok(s) = pulse_state.lock() {
                        if let Some(ref name) = s.default_sink_name {
                            debug!("Command Receiver: set_sink_mute_by_name {name} to {}", false);
                            introspect.set_sink_mute_by_name(name, false, Some(Box::new(|_| {})));
                        }
                    }
                    if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                        pending_refresh = true;
                    }
                }
                PulseCommand::NextDevice => {
                    let next_device = {
                        if let Ok(s) = pulse_state.lock() {
                            if let Some(current) = s.default_sink_index {
                                let current_pos = s.sinks.iter().position(|(idx, _)| *idx == current);
                                let next = current_pos.map(|pos| &s.sinks[(pos + 1) % s.sinks.len()]).or_else(|| s.sinks.first());
                                next.map(|(idx, name)| (*idx, name.clone()))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };
                    if let Some((next_idx, next_name)) = next_device {
                        debug!("Command Receiver: set_default_sink to {next_name}");
                        context.set_default_sink(&next_name, |_| {});
                        if let Ok(mut s) = pulse_state.lock() {
                            s.default_sink_index = Some(next_idx);
                            s.default_sink_name = Some(next_name);
                            s.pending_switch = true;
                        }
                    }
                    if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                        pending_refresh = true;
                    }
                }
                PulseCommand::PreviousDevice => {
                    let prev_device = {
                        if let Ok(s) = pulse_state.lock() {
                            if let Some(current) = s.default_sink_index {
                                let current_pos = s.sinks.iter().position(|(idx, _)| *idx == current);
                                let prev = current_pos
                                    .map(|pos| {
                                        let new_pos = if pos == 0 { s.sinks.len() - 1 } else { pos - 1 };
                                        &s.sinks[new_pos]
                                    })
                                    .or_else(|| s.sinks.last());
                                prev.map(|(idx, name)| (*idx, name.clone()))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };
                    if let Some((prev_idx, prev_name)) = prev_device {
                        debug!("Command Receiver: set_default_sink to {prev_name}");
                        context.set_default_sink(&prev_name, |_| {});
                        if let Ok(mut s) = pulse_state.lock() {
                            s.default_sink_index = Some(prev_idx);
                            s.default_sink_name = Some(prev_name);
                            s.pending_switch = true;
                        }
                    }
                    if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                        pending_refresh = true;
                    }
                }
                PulseCommand::RefreshStatus => {
                    if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                        pending_refresh = true;
                    }
                }
            },
            Err(RecvTimeoutError::Timeout) => {
                if pending_refresh && Instant::now().duration_since(last_refresh_time) > Duration::from_millis(50) {
                    pending_refresh = false;
                    refresh_and_broadcast(&mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender);
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    mainloop.stop();
    context.disconnect();
}

fn maybe_refresh(
    last_refresh_time: &mut Instant,
    mainloop: &mut Mainloop,
    introspect: &mut Introspector,
    pulse_state: &Arc<Mutex<PulseState>>,
    last_status: &Arc<Mutex<Option<AudioStatusMessage>>>,
    status_sender: &Sender<AudioStatusMessage>,
) -> bool {
    let now = Instant::now();
    if now.duration_since(*last_refresh_time) > Duration::from_millis(50) {
        *last_refresh_time = now;
        refresh_and_broadcast(mainloop, introspect, pulse_state, last_status, status_sender);
        true
    } else {
        false
    }
}

fn refresh_and_broadcast(
    mainloop: &mut Mainloop,
    introspect: &mut Introspector,
    pulse_state: &Arc<Mutex<PulseState>>,
    last_status: &Arc<Mutex<Option<AudioStatusMessage>>>,
    status_sender: &Sender<AudioStatusMessage>,
) {
    debug!("Audio Service: refresh_and_broadcast ");
    let Some(status) = query_status(mainloop, introspect, pulse_state) else {
        return;
    };
    let Ok(mut last) = last_status.lock() else {
        return;
    };
    if last.as_ref() != Some(&status) {
        debug!("Audio status updated: {status:?}");
        *last = Some(status.clone());
        let _ = status_sender.send(status);
    }
}

fn query_status(mainloop: &mut Mainloop, introspect: &mut Introspector, state: &Arc<Mutex<PulseState>>) -> Option<AudioStatusMessage> {
    debug!("Audio Service: query_status");
    let default_sink_name = Arc::new(Mutex::new(None::<String>));
    let ds = default_sink_name.clone();
    let ml: *mut Mainloop = mainloop;

    mainloop.lock();
    introspect.get_server_info(move |info: &ServerInfo| {
        *ds.lock().unwrap() = info.default_sink_name.as_deref().map(|s| s.to_string());
        unsafe {
            (*ml).signal(false);
        }
    });
    mainloop.wait();
    mainloop.unlock();

    debug!("Audio Service: query_status 2");

    let sinks_data = Arc::new(Mutex::new(Vec::new()));
    let sk = sinks_data.clone();
    let done = Arc::new(Mutex::new(false));
    let done_clone = done.clone();

    introspect.get_sink_info_list(move |result| match result {
        ListResult::Item(info) => {
            let volume_ratio = info.volume.avg().0 as f32 / Volume::NORMAL.0 as f32;
            sk.lock().unwrap().push((
                info.index,
                info.name.as_deref().unwrap_or("").to_string(),
                info.description.as_deref().unwrap_or("").to_string(),
                volume_ratio,
                info.mute,
                info.volume.len(),
            ));
        }
        ListResult::End => {
            *done_clone.lock().unwrap() = true;
        }
        ListResult::Error => {
            *done_clone.lock().unwrap() = true;
        }
    });

    // Poll with timeout instead of mainloop.wait() to avoid deadlock under rapid load.
    for _ in 0..50 {
        if *done.lock().unwrap() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    debug!("Audio Service: query_status 3");

    let mut default_name = default_sink_name.lock().unwrap().clone();
    let sinks = sinks_data.lock().unwrap();

    let mut output_devices = Vec::new();
    let mut active_device = None;
    let mut volume = 0.0f32;
    let mut is_muted = false;
    let mut active_channels = 2u8;
    let mut sink_list = Vec::new();

    // If a device switch was just commanded, PulseAudio may not have applied it yet.
    // Use the pending default from pulse_state instead of the stale value from PulseAudio.
    if let Ok(mut st) = state.lock() {
        if st.pending_switch {
            default_name = st.default_sink_name.clone();
            st.pending_switch = false;
        }
    }

    for (id, name, desc, vol, muted, ch) in sinks.iter() {
        let is_default = default_name.as_ref() == Some(name);
        let device = smearor_audio_model::AudioDevice {
            id: *id,
            name: desc.clone(),
            is_default,
        };
        if is_default {
            active_device = Some(device.clone());
            volume = *vol;
            is_muted = *muted;
            active_channels = *ch;
        }
        output_devices.push(device);
        sink_list.push((*id, name.clone()));
    }

    debug!("Audio Service: query_status 4");

    if let Ok(mut st) = state.lock() {
        st.default_sink_name = default_name;
        st.default_sink_index = active_device.as_ref().map(|d| d.id);
        st.volume = volume;
        st.mute = is_muted;
        st.channels = active_channels;
        st.sinks = sink_list;
    }

    debug!("Audio Service: query_status 5");

    Some(AudioStatusMessage::new(volume, is_muted, output_devices, Vec::new(), active_device))
}
