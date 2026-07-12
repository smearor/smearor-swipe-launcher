use crate::config::MprisServiceConfig;
use glib::MainContext;
use smearor_mpris_model::MprisCommandAction;
use smearor_mpris_model::MprisCommandMessage;
use smearor_mpris_model::MprisLoopStatus;
use smearor_mpris_model::MprisPlaybackStatus;
use smearor_mpris_model::MprisPlayerInfo;
use smearor_mpris_model::MprisStatusMessage;
use smearor_mpris_model::MprisTrackMetadata;
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

#[derive(Debug)]
pub enum MprisCommand {
    Play,
    Pause,
    TogglePlayPause,
    Stop,
    NextTrack,
    PreviousTrack,
    Seek(i64),
    SetPosition(i64),
    CycleLoop,
    ToggleShuffle,
    NextPlayer,
    PreviousPlayer,
    Raise,
    Quit,
    RefreshStatus,
}

#[derive(Clone, Debug, Default)]
struct MprisState {
    players: Vec<(String, String)>,
    active_player_index: Option<usize>,
    playback_status: MprisPlaybackStatus,
    metadata: Option<MprisTrackMetadata>,
    position: i64,
    loop_status: MprisLoopStatus,
    shuffle: bool,
    volume: f32,
    pending_switch: bool,
}

pub struct MprisService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    #[allow(unused)]
    pub config: MprisServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<MprisCommand>,
}

impl MprisService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        smearor_mpris_model::register_json_converters(core_context);

        let mpris_config: MprisServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel::<MprisCommand>();
        let (status_sender, mut status_receiver) = tokio::sync::mpsc::unbounded_channel::<MprisStatusMessage>();
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
        MainContext::default().spawn_local(async move {
            while let Some(status) = status_receiver.recv().await {
                let payload_ptr = Box::into_raw(Box::new(status)) as *mut core::ffi::c_void;
                let envelope = FfiEnvelope {
                    sender_id: stabby::string::String::from(meta_clone.id.clone()),
                    target_instance_id: stabby::string::String::from("*"),
                    topic: stabby::string::String::from(MprisStatusMessage::topic()),
                    type_id: MprisStatusMessage::TYPE_ID,
                    payload: payload_ptr,
                    destroy_payload: Some(destroy_mpris_status),
                    clone_payload: Some(clone_mpris_status),
                };
                if let Some(ctx) = &core_context_clone {
                    ctx.send_message(envelope);
                }
            }
        });

        Ok(MprisService {
            meta,
            core_context,
            config: mpris_config,
            command_sender,
        })
    }

    fn handle_play(&self) {
        let _ = self.command_sender.send(MprisCommand::Play);
    }
    fn handle_pause(&self) {
        let _ = self.command_sender.send(MprisCommand::Pause);
    }
    fn handle_toggle_play_pause(&self) {
        let _ = self.command_sender.send(MprisCommand::TogglePlayPause);
    }
    fn handle_stop(&self) {
        let _ = self.command_sender.send(MprisCommand::Stop);
    }
    fn handle_next_track(&self) {
        let _ = self.command_sender.send(MprisCommand::NextTrack);
    }
    fn handle_previous_track(&self) {
        let _ = self.command_sender.send(MprisCommand::PreviousTrack);
    }
    fn handle_seek(&self, offset: i64) {
        let _ = self.command_sender.send(MprisCommand::Seek(offset));
    }
    fn handle_set_position(&self, position: i64) {
        let _ = self.command_sender.send(MprisCommand::SetPosition(position));
    }
    fn handle_cycle_loop(&self) {
        let _ = self.command_sender.send(MprisCommand::CycleLoop);
    }
    fn handle_toggle_shuffle(&self) {
        let _ = self.command_sender.send(MprisCommand::ToggleShuffle);
    }
    fn handle_next_player(&self) {
        let _ = self.command_sender.send(MprisCommand::NextPlayer);
    }
    fn handle_previous_player(&self) {
        let _ = self.command_sender.send(MprisCommand::PreviousPlayer);
    }
    fn handle_raise(&self) {
        let _ = self.command_sender.send(MprisCommand::Raise);
    }
    fn handle_quit(&self) {
        let _ = self.command_sender.send(MprisCommand::Quit);
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
                if envelope.type_id == FfiEnvelopePayload::<MprisCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<MprisCommandMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
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

async fn run_mpris_async(
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

extern "C" fn clone_mpris_status(ptr: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let status = unsafe { &*(ptr as *const MprisStatusMessage) };
    Box::into_raw(Box::new(status.clone())) as *mut core::ffi::c_void
}

extern "C" fn destroy_mpris_status(ptr: *mut core::ffi::c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut MprisStatusMessage);
        }
    }
}
