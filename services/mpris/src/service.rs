use crate::config::MprisServiceConfig;
use abi_stable::std_types::RString;
use glib::ControlFlow;
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
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;
use tracing::debug;
use tracing::error;
use tracing::info;
use zbus::Connection;
use zbus::proxy;
use zbus::zvariant::OwnedValue;

#[proxy(interface = "org.mpris.MediaPlayer2.Player", default_path = "/org/mpris/MediaPlayer2")]
trait Player {
    #[zbus(property, name = "PlaybackStatus")]
    fn playback_status(&self) -> zbus::Result<String>;

    #[zbus(property, name = "Metadata")]
    fn metadata(&self) -> zbus::Result<HashMap<String, OwnedValue>>;

    #[zbus(property, name = "Position")]
    fn position(&self) -> zbus::Result<i64>;

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
    pub config: MprisServiceConfig,
    pub command_sender: Sender<MprisCommand>,
    pub status_receiver: Arc<Mutex<Receiver<MprisStatusMessage>>>,
}

impl MprisService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let mpris_config: MprisServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let (command_sender, command_receiver) = mpsc::channel::<MprisCommand>();
        let (status_sender, status_receiver) = mpsc::channel::<MprisStatusMessage>();
        let meta = PluginMeta::try_from(&config)?;
        std::thread::spawn(move || {
            run_mpris_thread(command_receiver, status_sender);
        });
        let service = MprisService {
            meta,
            core_context,
            config: mpris_config,
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
                        topic: RString::from(MprisStatusMessage::topic()),
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
}

impl MessageHandler<FfiEnvelopePayload<MprisCommandMessage>> for MprisService {
    fn handle_message(&self, message: FfiEnvelopePayload<MprisCommandMessage>, _sender_id: &str) {
        info!("MPRIS Service: received command {:?}", message.action);
        match message.action {
            MprisCommandAction::Play => self.handle_play(),
            MprisCommandAction::Pause => self.handle_pause(),
            MprisCommandAction::TogglePlayPause => self.handle_toggle_play_pause(),
            MprisCommandAction::Stop => self.handle_stop(),
            MprisCommandAction::NextTrack => self.handle_next_track(),
            MprisCommandAction::PreviousTrack => self.handle_previous_track(),
            MprisCommandAction::Seek => {
                if let Some(o) = message.seek_offset {
                    self.handle_seek(o);
                }
            }
            MprisCommandAction::SetPosition => {
                if let Some(p) = message.position {
                    self.handle_set_position(p);
                }
            }
            MprisCommandAction::CycleLoop => self.handle_cycle_loop(),
            MprisCommandAction::ToggleShuffle => self.handle_toggle_shuffle(),
            MprisCommandAction::NextPlayer => self.handle_next_player(),
            MprisCommandAction::PreviousPlayer => self.handle_previous_player(),
        }
    }
}

impl MessageBroadcaster<MprisStatusMessage> for MprisService {}
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

async fn find_players(conn: &Connection) -> Result<Vec<String>, zbus::Error> {
    let dbus = zbus::fdo::DBusProxy::new(conn).await?;
    let names = dbus.list_names().await?;
    Ok(names
        .into_iter()
        .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
        .map(|n| n.to_string())
        .collect())
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
    let position = proxy.position().await.unwrap_or(0);
    let title = extract_string(&metadata, "xesam:title").unwrap_or_default();
    let artist = extract_string_array(&metadata, "xesam:artist").join(", ");
    let album = extract_string(&metadata, "xesam:album").unwrap_or_default();
    let length = extract_string(&metadata, "mpris:length").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let art_url = extract_string(&metadata, "mpris:artUrl");
    let player_info = MprisPlayerInfo {
        bus_name: bus_name.to_string(),
        name: bus_name.trim_start_matches("org.mpris.MediaPlayer2.").to_string(),
        is_active: true,
    };
    let track_metadata = MprisTrackMetadata {
        title,
        artist,
        album,
        length,
        art_url,
    };
    Ok(MprisStatusMessage::new(
        true,
        Some(player_info.clone()),
        vec![player_info],
        parse_playback_status(&playback_status),
        Some(track_metadata),
        position,
        MprisLoopStatus::None,
        false,
        1.0,
    ))
}

async fn send_player_command(conn: &Connection, bus_name: &str, command: &MprisCommand) -> Result<(), zbus::Error> {
    let proxy = PlayerProxy::builder(conn).destination(bus_name)?.build().await?;
    match command {
        MprisCommand::Play | MprisCommand::Pause | MprisCommand::TogglePlayPause => proxy.play_pause().await?,
        MprisCommand::NextTrack => proxy.next().await?,
        MprisCommand::PreviousTrack => proxy.previous().await?,
        _ => {}
    }
    Ok(())
}

fn run_mpris_thread(command_receiver: Receiver<MprisCommand>, status_sender: Sender<MprisStatusMessage>) {
    debug!("MPRIS Service: starting MPRIS thread");
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            error!("MPRIS Service: failed to create tokio runtime: {e}");
            return;
        }
    };
    runtime.block_on(async {
        let conn = match Connection::session().await {
            Ok(c) => c,
            Err(e) => {
                error!("MPRIS Service: failed to connect to D-Bus session: {e}");
                return;
            }
        };
        let mut state = MprisState::default();
        let mut last_broadcast: Option<MprisStatusMessage> = None;
        let _ = status_sender.send(MprisStatusMessage::new(
            false,
            None,
            Vec::new(),
            MprisPlaybackStatus::Stopped,
            None,
            0,
            MprisLoopStatus::None,
            false,
            1.0,
        ));

        loop {
            match command_receiver.recv_timeout(Duration::from_millis(500)) {
                Ok(MprisCommand::NextPlayer) => {
                    if !state.players.is_empty() {
                        let new_idx = state.active_player_index.map(|i| (i + 1) % state.players.len()).unwrap_or(0);
                        state.active_player_index = Some(new_idx);
                        debug!("MPRIS Service: switched to player {}", state.players[new_idx].1);
                    }
                }
                Ok(MprisCommand::PreviousPlayer) => {
                    if !state.players.is_empty() {
                        let new_idx = state
                            .active_player_index
                            .map(|i| if i == 0 { state.players.len() - 1 } else { i - 1 })
                            .unwrap_or(0);
                        state.active_player_index = Some(new_idx);
                        debug!("MPRIS Service: switched to player {}", state.players[new_idx].1);
                    }
                }
                Ok(command) => {
                    debug!("MPRIS Service: received command {:?}", command);
                    if let Some(idx) = state.active_player_index {
                        if let Some((bus_name, _)) = state.players.get(idx) {
                            if let Err(e) = send_player_command(&conn, bus_name, &command).await {
                                error!("MPRIS Service: failed to send command to {bus_name}: {e}");
                            }
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    debug!("MPRIS Service: command channel disconnected, exiting thread");
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }

            let players = match find_players(&conn).await {
                Ok(p) => p,
                Err(e) => {
                    error!("MPRIS Service: failed to find players: {e}");
                    continue;
                }
            };

            let player_names: Vec<(String, String)> = players
                .iter()
                .map(|n| {
                    let display = n.trim_start_matches("org.mpris.MediaPlayer2.").to_string();
                    (n.clone(), display)
                })
                .collect();

            if player_names.is_empty() {
                state.players.clear();
                state.active_player_index = None;
                let no_player = MprisStatusMessage::new(false, None, Vec::new(), MprisPlaybackStatus::Stopped, None, 0, MprisLoopStatus::None, false, 1.0);
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
                            error!("MPRIS Service: failed to query player {bus_name}: {e}");
                            state.active_player_index = None;
                        }
                    }
                }
            }
        }
    });
    debug!("MPRIS Service: MPRIS thread exiting");
}
