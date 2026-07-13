use crate::mpris_command::MprisCommand;
use crate::mpris_state::MprisState;
use smearor_mpris_model::MprisLoopStatus;
use smearor_mpris_model::MprisPlaybackStatus;
use smearor_mpris_model::MprisPlayerInfo;
use smearor_mpris_model::MprisStatusMessage;
use smearor_mpris_model::MprisTrackMetadata;
use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;
use tracing::debug;
use tracing::error;
use tracing::trace;
use zbus::Connection;
use zbus::proxy;
use zbus::zvariant::OwnedValue;

#[proxy(interface = "org.mpris.MediaPlayer2", default_path = "/org/mpris/MediaPlayer2")]
trait MediaPlayer2 {
    fn raise(&self) -> zbus::Result<()>;
    fn quit(&self) -> zbus::Result<()>;
}

#[proxy(interface = "org.mpris.MediaPlayer2.Player", default_path = "/org/mpris/MediaPlayer2")]
trait Player {
    #[zbus(property, name = "PlaybackStatus")]
    fn playback_status(&self) -> zbus::Result<String>;

    #[zbus(property, name = "Metadata")]
    fn metadata(&self) -> zbus::Result<HashMap<String, OwnedValue>>;

    #[zbus(property, name = "Position")]
    fn position(&self) -> zbus::Result<i64>;

    fn play(&self) -> zbus::Result<()>;
    fn pause(&self) -> zbus::Result<()>;
    fn play_pause(&self) -> zbus::Result<()>;
    fn next(&self) -> zbus::Result<()>;
    fn previous(&self) -> zbus::Result<()>;
}

async fn find_players(conn: &Connection) -> Result<Vec<String>, zbus::Error> {
    let dbus = zbus::fdo::DBusProxy::new(conn).await?;
    let names = dbus.list_names().await?;
    let mpris_names: Vec<String> = names
        .into_iter()
        .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
        .map(|n| n.to_string())
        .collect();
    trace!("MPRIS Service: found players: {:?}", mpris_names);
    Ok(mpris_names)
}

fn extract_string(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    metadata.get(key).and_then(|v| match &**v {
        zbus::zvariant::Value::Str(s) => Some(s.to_string()),
        _ => None,
    })
}

fn extract_string_array(metadata: &HashMap<String, OwnedValue>, key: &str) -> Vec<String> {
    metadata
        .get(key)
        .map(|v| match &**v {
            zbus::zvariant::Value::Array(arr) => arr
                .iter()
                .filter_map(|v| match v {
                    zbus::zvariant::Value::Str(s) => Some(s.to_string()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        })
        .unwrap_or_default()
}

fn parse_playback_status(status: &str) -> MprisPlaybackStatus {
    match status {
        "Playing" => MprisPlaybackStatus::Playing,
        "Paused" => MprisPlaybackStatus::Paused,
        _ => MprisPlaybackStatus::Stopped,
    }
}

async fn query_player_status(conn: &Connection, bus_name: &str) -> Result<MprisStatusMessage, zbus::Error> {
    let proxy = PlayerProxy::builder(conn).destination(bus_name)?.build().await?;
    let playback_status = proxy.playback_status().await?;
    let metadata = proxy.metadata().await?;
    let position = match proxy.position().await {
        Ok(p) => p,
        Err(e) => {
            trace!("MPRIS Service: player {bus_name} does not support position query: {e}");
            0
        }
    };
    let title = stabby::string::String::from(extract_string(&metadata, "xesam:title").unwrap_or_default());
    let artist = stabby::string::String::from(extract_string_array(&metadata, "xesam:artist").join(", "));
    let album = stabby::string::String::from(extract_string(&metadata, "xesam:album").unwrap_or_default());
    let length = metadata.get("mpris:length").and_then(|v| v.downcast_ref::<i64>().ok()).unwrap_or(0);
    let art_url = match extract_string(&metadata, "mpris:artUrl") {
        Some(s) => stabby::option::Option::Some(stabby::string::String::from(s)),
        None => stabby::option::Option::None(),
    };
    let player_info = MprisPlayerInfo {
        bus_name: stabby::string::String::from(bus_name.to_string()),
        name: stabby::string::String::from(bus_name.trim_start_matches("org.mpris.MediaPlayer2.").to_string()),
        is_active: true,
    };
    let track_metadata = MprisTrackMetadata {
        title,
        artist,
        album,
        length,
        art_url,
    };
    let mut players = stabby::vec::Vec::new();
    players.push(player_info.clone());
    Ok(MprisStatusMessage::new(
        true,
        stabby::option::Option::Some(player_info.clone()),
        players,
        parse_playback_status(&playback_status),
        stabby::option::Option::Some(track_metadata),
        position,
        MprisLoopStatus::None,
        false,
        1.0,
    ))
}

async fn send_player_command(conn: &Connection, bus_name: &str, command: &MprisCommand, playback_status: &MprisPlaybackStatus) -> Result<(), zbus::Error> {
    match command {
        MprisCommand::Raise => {
            let proxy = MediaPlayer2Proxy::builder(conn).destination(bus_name)?.build().await?;
            proxy.raise().await?;
        }
        MprisCommand::Quit => {
            let proxy = MediaPlayer2Proxy::builder(conn).destination(bus_name)?.build().await?;
            proxy.quit().await?;
        }
        _ => {
            let proxy = PlayerProxy::builder(conn).destination(bus_name)?.build().await?;
            match command {
                MprisCommand::Play => proxy.play().await?,
                MprisCommand::Pause => proxy.pause().await?,
                MprisCommand::TogglePlayPause => {
                    if playback_status == &MprisPlaybackStatus::Playing {
                        proxy.pause().await?;
                    } else {
                        proxy.play().await?;
                    }
                }
                MprisCommand::Stop => proxy.pause().await?,
                MprisCommand::NextTrack => proxy.next().await?,
                MprisCommand::PreviousTrack => proxy.previous().await?,
                _ => {}
            }
        }
    }
    Ok(())
}

/// Runs the MPRIS async loop: discovers players, queries status, handles commands, and broadcasts updates.
pub(crate) async fn run_mpris_async(
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<MprisCommand>,
    status_sender: tokio::sync::mpsc::UnboundedSender<MprisStatusMessage>,
) {
    trace!("MPRIS Service: starting MPRIS async task");
    let conn = match Connection::session().await {
        Ok(c) => c,
        Err(e) => {
            error!("MPRIS Service: failed to connect to D-Bus session: {e}");
            return;
        }
    };
    let mut state = MprisState::default();
    let mut last_broadcast: Option<MprisStatusMessage> = None;
    let mut blocked_players: HashMap<String, Instant> = HashMap::new();
    const BLOCK_DURATION: Duration = Duration::from_secs(60);
    let _ = status_sender.send(MprisStatusMessage::new(
        false,
        stabby::option::Option::None(),
        stabby::vec::Vec::new(),
        MprisPlaybackStatus::Stopped,
        stabby::option::Option::None(),
        0,
        MprisLoopStatus::None,
        false,
        1.0,
    ));

    loop {
        let command = tokio::time::timeout(Duration::from_millis(500), command_receiver.recv()).await;
        match command {
            Ok(Some(MprisCommand::NextPlayer)) => {
                if !state.players.is_empty() {
                    let new_idx = state.active_player_index.map(|i| (i + 1) % state.players.len()).unwrap_or(0);
                    state.active_player_index = Some(new_idx);
                    trace!("MPRIS Service: switched to player {}", state.players[new_idx].1);
                }
            }
            Ok(Some(MprisCommand::PreviousPlayer)) => {
                if !state.players.is_empty() {
                    let new_idx = state
                        .active_player_index
                        .map(|i| if i == 0 { state.players.len() - 1 } else { i - 1 })
                        .unwrap_or(0);
                    state.active_player_index = Some(new_idx);
                    trace!("MPRIS Service: switched to player {}", state.players[new_idx].1);
                }
            }
            Ok(Some(command)) => {
                trace!("MPRIS Service: received command {:?}", command);
                if let Some(idx) = state.active_player_index {
                    if let Some((bus_name, _)) = state.players.get(idx) {
                        if let Err(e) = send_player_command(&conn, bus_name, &command, &state.playback_status).await {
                            error!("MPRIS Service: failed to send command to {bus_name}: {e}");
                        } else {
                            match command {
                                MprisCommand::Play => state.playback_status = MprisPlaybackStatus::Playing,
                                MprisCommand::Pause => state.playback_status = MprisPlaybackStatus::Paused,
                                MprisCommand::TogglePlayPause => {
                                    state.playback_status = if state.playback_status == MprisPlaybackStatus::Playing {
                                        MprisPlaybackStatus::Paused
                                    } else {
                                        MprisPlaybackStatus::Playing
                                    };
                                }
                                MprisCommand::Stop => state.playback_status = MprisPlaybackStatus::Stopped,
                                _ => {}
                            }
                        }
                    }
                }
            }
            Ok(None) => {
                trace!("MPRIS Service: command channel closed, exiting task");
                break;
            }
            Err(_) => {}
        }

        // Clean up expired blocked players
        let now = Instant::now();
        blocked_players.retain(|_, timestamp| now.duration_since(*timestamp) < BLOCK_DURATION);

        let players = match find_players(&conn).await {
            Ok(p) => p,
            Err(e) => {
                error!("MPRIS Service: failed to find players: {e}");
                continue;
            }
        };

        let player_names: Vec<(String, String)> = players
            .iter()
            .filter(|n| !blocked_players.contains_key(*n))
            .map(|n| {
                let display = n.trim_start_matches("org.mpris.MediaPlayer2.").to_string();
                (n.clone(), display)
            })
            .collect();

        trace!(
            "MPRIS Service: available players after filtering: {:?}",
            player_names.iter().map(|(_, d)| d.clone()).collect::<Vec<_>>()
        );

        if player_names.is_empty() {
            state.players.clear();
            state.active_player_index = None;
            let no_player = MprisStatusMessage::new(
                false,
                stabby::option::Option::None(),
                stabby::vec::Vec::new(),
                MprisPlaybackStatus::Stopped,
                stabby::option::Option::None(),
                0,
                MprisLoopStatus::None,
                false,
                1.0,
            );
            if last_broadcast.as_ref() != Some(&no_player) {
                let _ = status_sender.send(no_player.clone());
                last_broadcast = Some(no_player);
            }
            continue;
        }

        state.players = player_names.clone();
        if state.active_player_index.is_none() {
            state.active_player_index = Some(0);
        }
        if let Some(idx) = state.active_player_index {
            if idx >= state.players.len() {
                state.active_player_index = Some(0);
            }
        }

        if let Some(idx) = state.active_player_index {
            if let Some((bus_name, _)) = state.players.get(idx) {
                match query_player_status(&conn, bus_name).await {
                    Ok(status) => {
                        if last_broadcast.as_ref() != Some(&status) {
                            let _ = status_sender.send(status.clone());
                            last_broadcast = Some(status);
                        }
                    }
                    Err(e) => {
                        let error_str = e.to_string();
                        if error_str.contains("AccessDenied") {
                            trace!("MPRIS Service: blocking player {bus_name} for {}s", BLOCK_DURATION.as_secs());
                            blocked_players.insert(bus_name.clone(), Instant::now());
                            state.players.remove(idx);
                            if state.players.is_empty() {
                                state.active_player_index = None;
                            } else {
                                state.active_player_index = Some(idx % state.players.len());
                            }
                        } else {
                            error!("MPRIS Service: failed to query player {bus_name}: {e}");
                            state.active_player_index = None;
                        }
                    }
                }
            }
        }
    }
    debug!("MPRIS Service: MPRIS async task exiting");
}

/// FFI clone function for `MprisStatusMessage`.
pub(crate) extern "C" fn clone_mpris_status(ptr: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let status = unsafe { &*(ptr as *const MprisStatusMessage) };
    Box::into_raw(Box::new(status.clone())) as *mut core::ffi::c_void
}

/// FFI destroy function for `MprisStatusMessage`.
pub(crate) extern "C" fn destroy_mpris_status(ptr: *mut core::ffi::c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut MprisStatusMessage);
        }
    }
}
