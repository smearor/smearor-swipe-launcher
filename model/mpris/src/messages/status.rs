use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;

pub const TOPIC_STATUS: &str = "service.mpris.status";

/// Information about an available MPRIS player.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct MprisPlayerInfo {
    /// D-Bus bus name of the player (e.g. "org.mpris.MediaPlayer2.spotify")
    pub bus_name: String,
    /// Human-readable player name
    pub name: String,
    /// Whether this is the currently active player
    pub is_active: bool,
}

/// Current playback status of an MPRIS player.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum MprisPlaybackStatus {
    /// The player is actively playing
    Playing,
    /// The player is paused
    Paused,
    /// The player is stopped
    Stopped,
}

/// Loop mode of an MPRIS player.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum MprisLoopStatus {
    /// No looping
    None,
    /// Loop the current track
    Track,
    /// Loop the entire playlist
    Playlist,
}

/// Metadata of the currently playing track.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct MprisTrackMetadata {
    /// Track title
    pub title: String,
    /// Track artist(s)
    pub artist: String,
    /// Album name
    pub album: String,
    /// Track length in microseconds
    pub length: i64,
    /// Cover art URL or local path
    pub art_url: Option<String>,
}

/// Status message broadcast by the MPRIS service to all widgets.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct MprisStatusMessage {
    /// Whether any player is currently active
    pub has_player: bool,
    /// The currently active player
    pub active_player: Option<MprisPlayerInfo>,
    /// List of all available players
    pub players: Vec<MprisPlayerInfo>,
    /// Current playback status
    pub playback_status: MprisPlaybackStatus,
    /// Metadata of the current track
    pub metadata: Option<MprisTrackMetadata>,
    /// Current playback position in microseconds
    pub position: i64,
    /// Current loop mode
    pub loop_status: MprisLoopStatus,
    /// Whether shuffle is enabled
    pub shuffle: bool,
    /// Player volume (0.0 to 1.0)
    pub volume: f32,
}

impl MprisStatusMessage {
    pub fn new(
        has_player: bool,
        active_player: Option<MprisPlayerInfo>,
        players: Vec<MprisPlayerInfo>,
        playback_status: MprisPlaybackStatus,
        metadata: Option<MprisTrackMetadata>,
        position: i64,
        loop_status: MprisLoopStatus,
        shuffle: bool,
        volume: f32,
    ) -> Self {
        Self {
            has_player,
            active_player,
            players,
            playback_status,
            metadata,
            position,
            loop_status,
            shuffle,
            volume,
        }
    }
}

impl MessageTopic for MprisStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}
