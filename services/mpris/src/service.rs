use crate::config::MprisServiceConfig;
use abi_stable::std_types::RString;
use glib::ControlFlow;
use smearor_mpris_model::MprisCommandAction;
use smearor_mpris_model::MprisCommandMessage;
use smearor_mpris_model::MprisLoopStatus;
use smearor_mpris_model::MprisPlaybackStatus;
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
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;
use tracing::debug;
use tracing::error;
use tracing::info;

/// Internal commands sent to the MPRIS background thread.
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

/// Tracks the current MPRIS state to avoid feedback loops.
#[derive(Clone, Debug)]
struct MprisState {
    /// List of available players: (bus_name, name).
    players: Vec<(String, String)>,
    /// Index of the currently active player in the players list.
    active_player_index: Option<usize>,
    /// Current playback status.
    playback_status: MprisPlaybackStatus,
    /// Current track metadata.
    metadata: Option<MprisTrackMetadata>,
    /// Current position in microseconds.
    position: i64,
    /// Current loop status.
    loop_status: MprisLoopStatus,
    /// Whether shuffle is enabled.
    shuffle: bool,
    /// Player volume (0.0 to 1.0).
    volume: f32,
    /// Whether a player switch is in progress.
    pending_switch: bool,
}

impl Default for MprisState {
    fn default() -> Self {
        Self {
            players: Vec::new(),
            active_player_index: None,
            playback_status: MprisPlaybackStatus::Stopped,
            metadata: None,
            position: 0,
            loop_status: MprisLoopStatus::None,
            shuffle: false,
            volume: 1.0,
            pending_switch: false,
        }
    }
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
                if let Some(offset) = message.seek_offset {
                    self.handle_seek(offset);
                }
            }
            MprisCommandAction::SetPosition => {
                if let Some(position) = message.position {
                    self.handle_set_position(position);
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

fn run_mpris_thread(command_receiver: Receiver<MprisCommand>, status_sender: Sender<MprisStatusMessage>) {
    debug!("MPRIS Service: starting MPRIS thread");

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            error!("MPRIS Service: failed to create tokio runtime: {e}");
            return;
        }
    };

    let _ = runtime.block_on(async {
        // TODO: Implement D-Bus MPRIS connection using zbus.
        // For now, send an initial "no player" status and wait for commands.
        let initial_status = MprisStatusMessage::new(false, None, Vec::new(), MprisPlaybackStatus::Stopped, None, 0, MprisLoopStatus::None, false, 1.0);
        let _ = status_sender.send(initial_status);

        loop {
            match command_receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(command) => {
                    debug!("MPRIS Service: received command {:?}", command);
                    // TODO: Execute command via zbus D-Bus connection.
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // TODO: Poll MPRIS players for status updates.
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    debug!("MPRIS Service: command channel disconnected, exiting thread");
                    break;
                }
            }
        }
    });

    debug!("MPRIS Service: MPRIS thread exiting");
}
