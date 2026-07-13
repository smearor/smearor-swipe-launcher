use smearor_mpris_model::MprisLoopStatus;
use smearor_mpris_model::MprisPlaybackStatus;
use smearor_mpris_model::MprisTrackMetadata;

/// Tracks the current MPRIS player state for command execution.
#[derive(Clone, Debug, Default)]
pub(crate) struct MprisState {
    /// Available players: (bus_name, display_name).
    pub players: Vec<(String, String)>,
    /// Index of the currently active player.
    pub active_player_index: Option<usize>,
    /// Current playback status.
    pub playback_status: MprisPlaybackStatus,
    /// Current track metadata.
    pub metadata: Option<MprisTrackMetadata>,
    /// Current playback position in microseconds.
    pub position: i64,
    /// Current loop status.
    pub loop_status: MprisLoopStatus,
    /// Whether shuffle is enabled.
    pub shuffle: bool,
    /// Current volume (0.0 - 1.0).
    pub volume: f32,
    /// Whether a player switch is in progress.
    pub pending_switch: bool,
}
