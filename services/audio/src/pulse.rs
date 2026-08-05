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
                trace!("PulseAudio sink event detected, triggering status refresh");
                let _ = command_sender_clone.send(PulseCommand::RefreshStatus);
            }
        }
    })));

    // Trigger initial status broadcast so widgets get state immediately.
    let _ = command_sender.send(PulseCommand::RefreshStatus);

    let mut introspect = context.introspect();
    let mut last_refresh_time = Instant::now() - Duration::from_secs(1);
    let mut pending_refresh = false;
    let mut pre_duck_volume: Option<f32> = None;
    let mut pre_duck_sink_input_volumes: Vec<(u32, ChannelVolumes)> = Vec::new();
    let mut last_ducked_ratio: f32 = 0.2;

    loop {
        let command = tokio::time::timeout(Duration::from_millis(50), command_receiver.recv()).await;
        match command {
            Ok(Some(PulseCommand::VolumeUp)) => {
                trace!("Command Receiver: Volume up command received");
                if let Ok(s) = pulse_state.lock() {
                    if let Some(ref name) = s.default_sink_name {
                        let new_vol = (s.volume + 0.05).min(1.0);
                        let mut cv = ChannelVolumes::default();
                        cv.set(s.channels, Volume((Volume::NORMAL.0 as f32 * new_vol) as u32));
                        trace!("Command Receiver: set_sink_volume_by_name {name} to {new_vol}");
                        introspect.set_sink_volume_by_name(name, &cv, Some(Box::new(|_| {})));
                    }
                }
                if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                    pending_refresh = true;
                }
            }
            Ok(Some(PulseCommand::VolumeDown)) => {
                trace!("Command Receiver: Volume down command received");
                if let Ok(s) = pulse_state.lock() {
                    if let Some(ref name) = s.default_sink_name {
                        let new_vol = (s.volume - 0.05).max(0.0);
                        let mut cv = ChannelVolumes::default();
                        cv.set(s.channels, Volume((Volume::NORMAL.0 as f32 * new_vol) as u32));
                        trace!("Command Receiver: set_sink_volume_by_name {name} to {new_vol}");
                        introspect.set_sink_volume_by_name(name, &cv, Some(Box::new(|_| {})));
                    }
                }
                if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                    pending_refresh = true;
                }
            }
            Ok(Some(PulseCommand::SetVolume(volume))) => {
                trace!("Command Receiver: Set volume command received");
                if let Ok(s) = pulse_state.lock() {
                    if let Some(ref name) = s.default_sink_name {
                        let new_vol = volume.clamp(0.0, 1.0);
                        let mut cv = ChannelVolumes::default();
                        cv.set(s.channels, Volume((Volume::NORMAL.0 as f32 * new_vol) as u32));
                        trace!("Command Receiver: set_sink_volume_by_name {name} to {new_vol}");
                        introspect.set_sink_volume_by_name(name, &cv, Some(Box::new(|_| {})));
                    }
                }
                if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                    pending_refresh = true;
                }
            }
            Ok(Some(PulseCommand::ToggleMute)) => {
                trace!("Command Receiver: toggle mute command received");
                if let Ok(s) = pulse_state.lock() {
                    if let Some(ref name) = s.default_sink_name {
                        trace!("Command Receiver: set_sink_mute_by_name {name} to {}", !s.mute);
                        introspect.set_sink_mute_by_name(name, !s.mute, Some(Box::new(|_| {})));
                    }
                }
                if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                    pending_refresh = true;
                }
            }
            Ok(Some(PulseCommand::Mute)) => {
                trace!("Command Receiver: mute command received");
                if let Ok(s) = pulse_state.lock() {
                    if let Some(ref name) = s.default_sink_name {
                        trace!("Command Receiver: set_sink_mute_by_name {name} to {}", true);
                        introspect.set_sink_mute_by_name(name, true, Some(Box::new(|_| {})));
                    }
                }
                if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                    pending_refresh = true;
                }
            }
            Ok(Some(PulseCommand::Unmute)) => {
                trace!("Command Receiver: unmute command received");
                if let Ok(s) = pulse_state.lock() {
                    if let Some(ref name) = s.default_sink_name {
                        trace!("Command Receiver: set_sink_mute_by_name {name} to {}", false);
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
                    trace!("Command Receiver: set_default_sink to {next_name}");
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
                    trace!("Command Receiver: set_default_sink to {prev_name}");
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
                let now = Instant::now();
                if now.duration_since(last_refresh_time) > Duration::from_millis(50) {
                    last_refresh_time = now;
                    force_refresh_and_broadcast(&mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender);
                } else {
                    pending_refresh = true;
                }
            }
            Ok(Some(PulseCommand::DuckVolume(target))) => {
                trace!("Command Receiver: DuckVolume to {}", target);
                let ducked = target.clamp(0.0, 1.0);
                last_ducked_ratio = ducked;
                let duck_notification_sounds = _config.duck_notification_sounds;

                // Save pre-duck sink input volumes for later restore.
                pre_duck_sink_input_volumes.clear();
                let volumes_clone = Arc::new(Mutex::new(Vec::<(u32, ChannelVolumes)>::new()));
                let volumes_clone_inner = volumes_clone.clone();

                introspect.get_sink_input_info_list(move |list_result| {
                    if let ListResult::Item(info) = list_result {
                        let role = info.proplist.get_str("media.role").unwrap_or_default().to_string();
                        let should_duck = is_duckable_role(&role, duck_notification_sounds);
                        if should_duck {
                            if let Ok(mut vols) = volumes_clone_inner.lock() {
                                vols.push((info.index, info.volume));
                            }
                            // Volume will be set after the callback completes.
                        }
                    }
                });

                // Wait briefly for the callback to populate volumes, then apply ducking.
                tokio::time::sleep(Duration::from_millis(10)).await;
                if let Ok(vols) = volumes_clone.lock() {
                    pre_duck_sink_input_volumes = vols.iter().map(|(i, v)| (*i, v.clone())).collect();
                    for (index, _) in vols.iter() {
                        let mut cv = ChannelVolumes::default();
                        if let Ok(s) = pulse_state.lock() {
                            cv.set(s.channels, Volume((Volume::NORMAL.0 as f32 * ducked) as u32));
                        }
                        introspect.set_sink_input_volume(*index, &cv, Some(Box::new(|_| {})));
                    }
                }

                // Also duck the sink master as fallback for streams without media.role.
                if pre_duck_volume.is_none() {
                    if let Ok(s) = pulse_state.lock() {
                        pre_duck_volume = Some(s.volume);
                    }
                }
                if let Ok(s) = pulse_state.lock() {
                    if let Some(ref name) = s.default_sink_name {
                        let mut cv = ChannelVolumes::default();
                        cv.set(s.channels, Volume((Volume::NORMAL.0 as f32 * ducked) as u32));
                        introspect.set_sink_volume_by_name(name, &cv, Some(Box::new(|_| {})));
                    }
                }
                if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                    pending_refresh = true;
                }
            }
            Ok(Some(PulseCommand::FadeRestoreVolume { target, ramp_ms })) => {
                trace!("Command Receiver: FadeRestoreVolume to {} over {} ms", target, ramp_ms);
                let saved_vol = pre_duck_volume.take();
                let saved_sink_input_volumes = std::mem::take(&mut pre_duck_sink_input_volumes);
                let start_vol = if let Some(v) = saved_vol {
                    v
                } else if let Ok(s) = pulse_state.lock() {
                    s.volume
                } else {
                    1.0
                };
                let target_vol = target.clamp(0.0, 1.0);
                let start_clamped = start_vol.clamp(0.0, 1.0);

                if ramp_ms == 0 || (start_clamped - target_vol).abs() < 0.001 {
                    // No ramp needed — instant restore.
                    if let Ok(s) = pulse_state.lock() {
                        if let Some(ref name) = s.default_sink_name {
                            let mut cv = ChannelVolumes::default();
                            cv.set(s.channels, Volume((Volume::NORMAL.0 as f32 * target_vol) as u32));
                            introspect.set_sink_volume_by_name(name, &cv, Some(Box::new(|_| {})));
                        }
                    }
                    // Restore individual sink input volumes.
                    for (index, original_cv) in &saved_sink_input_volumes {
                        introspect.set_sink_input_volume(*index, original_cv, Some(Box::new(|_| {})));
                    }
                    if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                        pending_refresh = true;
                    }
                } else {
                    // Linear fade ramp: stepwise volume updates.
                    let step_count = (ramp_ms / 50).max(1);
                    let step_duration = Duration::from_millis(ramp_ms / step_count);
                    let volume_delta = (target_vol - start_clamped) / step_count as f32;

                    for step in 1..=step_count {
                        let current_vol = (start_clamped + volume_delta * step as f32).clamp(0.0, 1.0);
                        if let Ok(s) = pulse_state.lock() {
                            if let Some(ref name) = s.default_sink_name {
                                let mut cv = ChannelVolumes::default();
                                cv.set(s.channels, Volume((Volume::NORMAL.0 as f32 * current_vol) as u32));
                                introspect.set_sink_volume_by_name(name, &cv, Some(Box::new(|_| {})));
                            }
                        }
                        // Fade individual sink input volumes back to their original values.
                        let fade_ratio = if step == step_count { 1.0 } else { step as f32 / step_count as f32 };
                        for (index, original_cv) in &saved_sink_input_volumes {
                            let mut cv = original_cv.clone();
                            let current_scale = last_ducked_ratio + (1.0 - last_ducked_ratio) * fade_ratio;
                            let scaled_vol = Volume((Volume::NORMAL.0 as f32 * current_scale) as u32);
                            cv.scale(scaled_vol);
                            introspect.set_sink_input_volume(*index, &cv, Some(Box::new(|_| {})));
                        }
                        // Wait for the step duration before next step.
                        tokio::time::sleep(step_duration).await;
                        // Check for incoming commands during the ramp (allows re-duck interruption).
                        if let Ok(Some(cmd)) = tokio::time::timeout(Duration::from_millis(0), command_receiver.recv()).await {
                            trace!("Command Receiver: fade ramp interrupted by {:?}", cmd);
                            match cmd {
                                PulseCommand::DuckVolume(t) => {
                                    if pre_duck_volume.is_none() {
                                        if let Ok(s) = pulse_state.lock() {
                                            pre_duck_volume = Some(s.volume);
                                        }
                                    }
                                    if let Ok(s) = pulse_state.lock() {
                                        if let Some(ref name) = s.default_sink_name {
                                            let ducked = t.clamp(0.0, 1.0);
                                            let mut cv = ChannelVolumes::default();
                                            cv.set(s.channels, Volume((Volume::NORMAL.0 as f32 * ducked) as u32));
                                            introspect.set_sink_volume_by_name(name, &cv, Some(Box::new(|_| {})));
                                        }
                                    }
                                    break;
                                }
                                other => {
                                    let _ = command_sender.send(other);
                                    break;
                                }
                            }
                        }
                    }
                    if !maybe_refresh(&mut last_refresh_time, &mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender) {
                        pending_refresh = true;
                    }
                }
            }
            Err(_) => {
                if pending_refresh && Instant::now().duration_since(last_refresh_time) > Duration::from_millis(50) {
                    pending_refresh = false;
                    force_refresh_and_broadcast(&mut mainloop, &mut introspect, &pulse_state, &last_status, &status_sender);
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

fn force_refresh_and_broadcast(
    mainloop: &mut Mainloop,
    introspect: &mut Introspector,
    pulse_state: &Arc<Mutex<PulseState>>,
    last_status: &Arc<Mutex<Option<AudioStatusMessage>>>,
    status_sender: &tokio::sync::mpsc::UnboundedSender<AudioStatusMessage>,
) {
    trace!("Audio Service: force_refresh_and_broadcast");
    let Some(status) = query_status(mainloop, introspect, pulse_state) else {
        return;
    };
    if let Ok(mut last) = last_status.lock() {
        *last = Some(status.clone());
    }
    let _ = status_sender.send(status);
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

/// Returns true if a sink input with the given `media.role` should be ducked.
///
/// Media roles that are always ducked: `music`, `video`, `game`, `movie`.
/// Roles that are never ducked: `phone`, `tts`, `a11y`, `production`.
/// The `notification` and `event` roles are ducked only when
/// `duck_notification_sounds` is true.
fn is_duckable_role(role: &str, duck_notification_sounds: bool) -> bool {
    match role {
        "music" | "video" | "game" | "movie" => true,
        "notification" | "event" => duck_notification_sounds,
        _ => true, // Unknown roles default to ducked (matches sink-master fallback behavior).
    }
}
