use crate::mpris_command::MprisCommand;
use crate::mpris_state::MprisState;
use crate::mpris_state::PlayerEntry;
use futures_util::StreamExt;
use smearor_mpris_model::MprisLoopStatus;
use smearor_mpris_model::MprisPlaybackStatus;
use smearor_mpris_model::MprisPlayerInfo;
use smearor_mpris_model::MprisStatusMessage;
use smearor_mpris_model::MprisTrackMetadata;
use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;
use tokio::task::JoinHandle;
use tracing::debug;
use tracing::error;
use tracing::trace;
use zbus::Connection;
use zbus::fdo::PropertiesProxy;
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

    #[zbus(property, name = "LoopStatus")]
    fn loop_status(&self) -> zbus::Result<String>;
    #[zbus(property, name = "LoopStatus")]
    fn set_loop_status(&self, value: &str) -> zbus::Result<()>;

    #[zbus(property, name = "Shuffle")]
    fn shuffle(&self) -> zbus::Result<bool>;
    #[zbus(property, name = "Shuffle")]
    fn set_shuffle(&self, value: bool) -> zbus::Result<()>;

    fn seek(&self, offset: i64) -> zbus::Result<()>;
    fn set_position(&self, track_id: &str, position: i64) -> zbus::Result<()>;

    fn play(&self) -> zbus::Result<()>;
    fn pause(&self) -> zbus::Result<()>;
    fn stop(&self) -> zbus::Result<()>;
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

async fn query_playback_status(conn: &Connection, bus_name: &str) -> Result<MprisPlaybackStatus, zbus::Error> {
    let proxy = PlayerProxy::builder(conn).destination(bus_name)?.build().await?;
    let status = proxy.playback_status().await?;
    Ok(parse_playback_status(&status))
}

/// Listens for D-Bus `PropertiesChanged` signals from a player and triggers a status refresh.
async fn listen_for_properties_changed(conn: Connection, bus_name: String, refresh_tx: tokio::sync::mpsc::UnboundedSender<()>) {
    let props = match PropertiesProxy::new(&conn, bus_name.as_str(), "/org/mpris/MediaPlayer2").await {
        Ok(p) => p,
        Err(e) => {
            trace!("MPRIS Service: failed to create PropertiesProxy for {bus_name}: {e}");
            return;
        }
    };
    let mut stream = match props.receive_properties_changed().await {
        Ok(s) => s,
        Err(e) => {
            trace!("MPRIS Service: failed to subscribe to PropertiesChanged for {bus_name}: {e}");
            return;
        }
    };
    trace!("MPRIS Service: listening for PropertiesChanged signals from {bus_name}");
    while let Some(_msg) = stream.next().await {
        trace!("MPRIS Service: received PropertiesChanged signal from {bus_name}");
        let _ = refresh_tx.send(());
    }
    trace!("MPRIS Service: PropertiesChanged stream ended for {bus_name}");
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
    let length = match metadata.get("mpris:length") {
        Some(v) => {
            if let Ok(val) = v.downcast_ref::<i64>() {
                val
            } else if let Ok(val) = v.downcast_ref::<u64>() {
                val as i64
            } else {
                trace!("MPRIS Service: mpris:length has unexpected type: {:?}", &**v);
                0
            }
        }
        None => {
            trace!("MPRIS Service: mpris:length not present in metadata for {bus_name}");
            0
        }
    };
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
                MprisCommand::Stop => proxy.stop().await?,
                MprisCommand::NextTrack => proxy.next().await?,
                MprisCommand::PreviousTrack => proxy.previous().await?,
                MprisCommand::Seek(offset) => proxy.seek(*offset).await?,
                MprisCommand::SetPosition(pos) => {
                    let track_id = "/org/mpris/MediaPlayer2";
                    proxy.set_position(track_id, *pos).await?;
                }
                MprisCommand::CycleLoop => {
                    let current = proxy.loop_status().await.unwrap_or_else(|_| "None".to_string());
                    let next = match current.as_str() {
                        "None" => "Track",
                        "Track" => "Playlist",
                        _ => "None",
                    };
                    proxy.set_loop_status(next).await?;
                }
                MprisCommand::ToggleShuffle => {
                    let current = proxy.shuffle().await.unwrap_or(false);
                    proxy.set_shuffle(!current).await?;
                }
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
    let mut last_refresh_time = Instant::now() - Duration::from_secs(1);
    let mut blocked_players: HashMap<String, Instant> = HashMap::new();
    let (refresh_tx, mut refresh_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut signal_listener: Option<JoinHandle<()>> = None;
    let mut signal_listener_bus_name: Option<String> = None;
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
        let command = tokio::time::timeout(Duration::from_millis(500), async {
            tokio::select! {
                cmd = command_receiver.recv() => cmd,
                _ = refresh_rx.recv() => Some(MprisCommand::RefreshStatus),
            }
        })
        .await;
        match command {
            Ok(Some(MprisCommand::NextPlayer)) => {
                if !state.players.is_empty() {
                    let new_idx = state.active_player_index.map(|i| (i + 1) % state.players.len()).unwrap_or(0);
                    state.active_player_index = Some(new_idx);
                    trace!("MPRIS Service: switched to player {}", state.players[new_idx].display_name);
                }
            }
            Ok(Some(MprisCommand::PreviousPlayer)) => {
                if !state.players.is_empty() {
                    let new_idx = state
                        .active_player_index
                        .map(|i| if i == 0 { state.players.len() - 1 } else { i - 1 })
                        .unwrap_or(0);
                    state.active_player_index = Some(new_idx);
                    trace!("MPRIS Service: switched to player {}", state.players[new_idx].display_name);
                }
            }
            Ok(Some(MprisCommand::RefreshStatus)) => {
                let now = Instant::now();
                if now.duration_since(last_refresh_time) > Duration::from_millis(50) {
                    last_refresh_time = now;
                    last_broadcast = None;
                    trace!("MPRIS Service: forcing status refresh");
                }
            }
            Ok(Some(command)) => {
                trace!("MPRIS Service: received command {:?}", command);
                if let Some(idx) = state.active_player_index {
                    if let Some(player) = state.players.get(idx) {
                        if let Err(e) = send_player_command(&conn, &player.bus_name, &command, &state.playback_status).await {
                            error!("MPRIS Service: failed to send command to {}: {e}", player.bus_name);
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

        let player_names: Vec<PlayerEntry> = players
            .iter()
            .filter(|n| !blocked_players.contains_key(*n))
            .map(|n| PlayerEntry {
                bus_name: n.clone(),
                display_name: n.trim_start_matches("org.mpris.MediaPlayer2.").to_string(),
            })
            .collect();

        trace!(
            "MPRIS Service: available players after filtering: {:?}",
            player_names.iter().map(|p| p.display_name.clone()).collect::<Vec<_>>()
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

        // Detect new players that weren't in the previous list
        let previous_names: Vec<String> = state.players.iter().map(|p| p.bus_name.clone()).collect();
        let new_player_names: Vec<(usize, PlayerEntry)> = player_names
            .iter()
            .enumerate()
            .filter(|(_, p)| !previous_names.contains(&p.bus_name))
            .map(|(i, p)| (i, p.clone()))
            .collect();

        state.players = player_names.clone();

        // Auto-select a playing player on startup or when the index is out of bounds
        let needs_initial_selection = state.active_player_index.is_none() || state.active_player_index.is_some_and(|idx| idx >= state.players.len());
        if needs_initial_selection {
            let mut playing_idx = None;
            for (i, player) in player_names.iter().enumerate() {
                match query_playback_status(&conn, &player.bus_name).await {
                    Ok(MprisPlaybackStatus::Playing) => {
                        playing_idx = Some(i);
                        break;
                    }
                    _ => {}
                }
            }
            state.active_player_index = Some(playing_idx.unwrap_or(0));
            if let Some(idx) = playing_idx {
                trace!("MPRIS Service: auto-selected playing player on startup: {}", player_names[idx].display_name);
            }
        } else if !new_player_names.is_empty() && state.playback_status != MprisPlaybackStatus::Playing {
            // Current player is not playing; check if any new player is playing
            for (_, player) in &new_player_names {
                match query_playback_status(&conn, &player.bus_name).await {
                    Ok(MprisPlaybackStatus::Playing) => {
                        if let Some(idx) = player_names.iter().position(|p| p.bus_name == player.bus_name) {
                            state.active_player_index = Some(idx);
                            trace!("MPRIS Service: auto-switched to new playing player: {}", player.display_name);
                            last_broadcast = None;
                        }
                        break;
                    }
                    _ => {}
                }
            }
        }

        // If the active player is not playing, check if any other player started playing
        if state.playback_status != MprisPlaybackStatus::Playing {
            if let Some(active_idx) = state.active_player_index {
                for (i, player) in player_names.iter().enumerate() {
                    if i == active_idx {
                        continue;
                    }
                    match query_playback_status(&conn, &player.bus_name).await {
                        Ok(MprisPlaybackStatus::Playing) => {
                            state.active_player_index = Some(i);
                            trace!("MPRIS Service: auto-switched to playing player: {}", player.display_name);
                            last_broadcast = None;
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(idx) = state.active_player_index {
            if let Some(player) = state.players.get(idx) {
                let bus_name = &player.bus_name;
                // (Re)spawn the PropertiesChanged signal listener when the active player changes
                if signal_listener_bus_name.as_deref() != Some(bus_name.as_str()) {
                    if let Some(handle) = signal_listener.take() {
                        handle.abort();
                    }
                    let conn_clone = conn.clone();
                    let refresh_tx_clone = refresh_tx.clone();
                    let bus_name_clone = bus_name.clone();
                    signal_listener = Some(tokio::spawn(async move {
                        listen_for_properties_changed(conn_clone, bus_name_clone, refresh_tx_clone).await;
                    }));
                    signal_listener_bus_name = Some(bus_name.clone());
                }
                match query_player_status(&conn, bus_name).await {
                    Ok(status) => {
                        state.playback_status = status.playback_status.clone();
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
