use smearor_mpris_model::MprisPlaybackStatus;

/// A discovered MPRIS player with its D-Bus bus name and human-readable display name.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct PlayerEntry {
    /// D-Bus bus name (e.g. "org.mpris.MediaPlayer2.spotify").
    pub bus_name: String,
    /// Human-readable player name (bus name without the "org.mpris.MediaPlayer2." prefix).
    pub display_name: String,
}

/// Tracks the current MPRIS player state for command execution.
#[derive(Clone, Debug, Default)]
pub(crate) struct MprisState {
    /// Available players.
    pub players: Vec<PlayerEntry>,
    /// Index of the currently active player.
    pub active_player_index: Option<usize>,
    /// Current playback status.
    pub playback_status: MprisPlaybackStatus,
}
