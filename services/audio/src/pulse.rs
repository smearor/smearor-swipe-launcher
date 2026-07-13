use crate::config::AudioServiceConfig;
use crate::pulse_command::PulseCommand;
use crate::pulse_state::PulseState;
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
use smearor_audio_model::AudioStatusMessage;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;
use tracing::debug;
use tracing::error;
use tracing::trace;

/// Main asynchronous loop interacting with PulseAudio.
///
/// Receives commands via `command_receiver`, dispatches them to PulseAudio,
/// and broadcasts status updates via `status_sender`.
pub async fn run_pulse_async(
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<PulseCommand>,
    command_sender: tokio::sync::mpsc::UnboundedSender<PulseCommand>,
    status_sender: tokio::sync::mpsc::UnboundedSender<AudioStatusMessage>,
    _config: AudioServiceConfig,
    last_status: Arc<Mutex<Option<AudioStatusMessage>>>,
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
    trace!("Audio Service: PulseAudio context ready");

    let pulse_state = Arc::new(Mutex::new(PulseState::default()));
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
        let command = tokio::time::timeout(Duration::from_millis(50), command_receiver.recv()).await;
        match command {
            Ok(Some(PulseCommand::VolumeUp)) => {
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
            Ok(Some(PulseCommand::VolumeDown)) => {
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
            Ok(Some(PulseCommand::SetVolume(volume))) => {
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
            Ok(Some(PulseCommand::ToggleMute)) => {
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
            Ok(Some(PulseCommand::Mute)) => {
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
            Ok(Some(PulseCommand::Unmute)) => {
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
            Ok(Some(PulseCommand::NextDevice)) => {
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
            Ok(Some(PulseCommand::PreviousDevice)) => {
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
            Ok(Some(PulseCommand::RefreshStatus)) => {
                if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                    pending_refresh = true;
                }
            }
            Err(_) => {
                if pending_refresh && Instant::now().duration_since(last_refresh_time) > Duration::from_millis(50) {
                    pending_refresh = false;
                    refresh_and_broadcast(&mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender);
                }
            }
            Ok(None) => break,
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
    status_sender: &tokio::sync::mpsc::UnboundedSender<AudioStatusMessage>,
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
    status_sender: &tokio::sync::mpsc::UnboundedSender<AudioStatusMessage>,
) {
    trace!("Audio Service: refresh_and_broadcast ");
    let Some(status) = query_status(mainloop, introspect, pulse_state) else {
        return;
    };
    let Ok(mut last) = last_status.lock() else {
        return;
    };
    if last.as_ref() != Some(&status) {
        trace!("Audio status updated: {status:?}");
        *last = Some(status.clone());
        let _ = status_sender.send(status);
    }
}

fn query_status(mainloop: &mut Mainloop, introspect: &mut Introspector, state: &Arc<Mutex<PulseState>>) -> Option<AudioStatusMessage> {
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

    let mut default_name = default_sink_name.lock().unwrap().clone();
    let sinks = sinks_data.lock().unwrap();

    let mut output_devices = stabby::vec::Vec::new();
    let mut active_device: stabby::option::Option<smearor_audio_model::AudioDevice> = stabby::option::Option::None();
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
            name: stabby::string::String::from(desc.clone()),
            is_default,
        };
        if is_default {
            active_device = stabby::option::Option::Some(device.clone());
            volume = *vol;
            is_muted = *muted;
            active_channels = *ch;
        }
        output_devices.push(device);
        sink_list.push((*id, name.clone()));
    }

    if let Ok(mut st) = state.lock() {
        st.default_sink_name = default_name;
        st.default_sink_index = active_device.as_ref().map(|d| d.id);
        st.volume = volume;
        st.mute = is_muted;
        st.channels = active_channels;
        st.sinks = sink_list;
    }

    Some(AudioStatusMessage::new(volume, is_muted, output_devices, stabby::vec::Vec::new(), active_device))
}

/// FFI clone function for `AudioStatusMessage` payload.
pub extern "C" fn clone_audio_status(ptr: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let status = unsafe { &*(ptr as *const AudioStatusMessage) };
    Box::into_raw(Box::new(status.clone())) as *mut core::ffi::c_void
}

/// FFI destroy function for `AudioStatusMessage` payload.
pub extern "C" fn destroy_audio_status(ptr: *mut core::ffi::c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut AudioStatusMessage);
        }
    }
}
