use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;

pub const TOPIC_COMMAND: &str = "service.mpris.command";

/// Actions that can be sent from the widget to the MPRIS service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum MprisCommandAction {
    #[default]
    /// Start or resume playback
    Play,
    /// Pause playback
    Pause,
    /// Toggle between play and pause
    TogglePlayPause,
    /// Stop playback
    Stop,
    /// Skip to the next track
    NextTrack,
    /// Go back to the previous track
    PreviousTrack,
    /// Seek forward or backward by an offset in microseconds
    Seek,
    /// Set the playback position to an absolute value in microseconds
    SetPosition,
    /// Cycle loop mode (None -> Track -> Playlist)
    CycleLoop,
    /// Toggle shuffle on/off
    ToggleShuffle,
    /// Switch to the next active player
    NextPlayer,
    /// Switch to the previous active player
    PreviousPlayer,
}

/// Command message sent from the MPRIS widget to the MPRIS service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MprisCommandMessage {
    /// The action to execute
    pub action: MprisCommandAction,
    /// Optional seek offset in microseconds (positive or negative)
    pub seek_offset: Option<i64>,
    /// Optional absolute position in microseconds
    pub position: Option<i64>,
    /// Optional player bus name to target a specific player
    pub player_bus_name: Option<String>,
}

impl MprisCommandMessage {
    pub fn new(action: MprisCommandAction, seek_offset: Option<i64>, position: Option<i64>, player_bus_name: Option<String>) -> Self {
        Self {
            action,
            seek_offset,
            position,
            player_bus_name,
        }
    }

    pub fn play() -> Self {
        Self::new(MprisCommandAction::Play, None, None, None)
    }

    pub fn pause() -> Self {
        Self::new(MprisCommandAction::Pause, None, None, None)
    }

    pub fn toggle_play_pause() -> Self {
        Self::new(MprisCommandAction::TogglePlayPause, None, None, None)
    }

    pub fn stop() -> Self {
        Self::new(MprisCommandAction::Stop, None, None, None)
    }

    pub fn next_track() -> Self {
        Self::new(MprisCommandAction::NextTrack, None, None, None)
    }

    pub fn previous_track() -> Self {
        Self::new(MprisCommandAction::PreviousTrack, None, None, None)
    }

    pub fn seek(offset: i64) -> Self {
        Self::new(MprisCommandAction::Seek, Some(offset), None, None)
    }

    pub fn set_position(position: i64) -> Self {
        Self::new(MprisCommandAction::SetPosition, None, Some(position), None)
    }

    pub fn cycle_loop() -> Self {
        Self::new(MprisCommandAction::CycleLoop, None, None, None)
    }

    pub fn toggle_shuffle() -> Self {
        Self::new(MprisCommandAction::ToggleShuffle, None, None, None)
    }

    pub fn next_player() -> Self {
        Self::new(MprisCommandAction::NextPlayer, None, None, None)
    }

    pub fn previous_player() -> Self {
        Self::new(MprisCommandAction::PreviousPlayer, None, None, None)
    }
}

impl MessageTopic for MprisCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}
