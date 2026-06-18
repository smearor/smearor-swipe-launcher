use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_STATUS: &str = "service.mpris.status";

/// Information about an available MPRIS player.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, PartialEq)]
pub struct MprisPlayerInfo {
    /// D-Bus bus name of the player (e.g. "org.mpris.MediaPlayer2.spotify")
    pub bus_name: stabby::string::String,
    /// Human-readable player name
    pub name: stabby::string::String,
    /// Whether this is the currently active player
    pub is_active: bool,
}

/// Current playback status of an MPRIS player.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq)]
pub enum MprisPlaybackStatus {
    /// The player is actively playing
    Playing,
    /// The player is paused
    Paused,
    /// The player is stopped
    #[default]
    Stopped,
}

/// Loop mode of an MPRIS player.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq)]
pub enum MprisLoopStatus {
    /// No looping
    #[default]
    None,
    /// Loop the current track
    Track,
    /// Loop the entire playlist
    Playlist,
}

/// Metadata of the currently playing track.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, PartialEq)]
pub struct MprisTrackMetadata {
    /// Track title
    pub title: stabby::string::String,
    /// Track artist(s)
    pub artist: stabby::string::String,
    /// Album name
    pub album: stabby::string::String,
    /// Track length in microseconds
    pub length: i64,
    /// Cover art URL or local path
    pub art_url: stabby::option::Option<stabby::string::String>,
}

/// Status message broadcast by the MPRIS service to all widgets.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, PartialEq)]
pub struct MprisStatusMessage {
    /// Whether any player is currently active
    pub has_player: bool,
    /// The currently active player
    pub active_player: stabby::option::Option<MprisPlayerInfo>,
    /// List of all available players
    pub players: stabby::vec::Vec<MprisPlayerInfo>,
    /// Current playback status
    pub playback_status: MprisPlaybackStatus,
    /// Metadata of the current track
    pub metadata: stabby::option::Option<MprisTrackMetadata>,
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
        active_player: stabby::option::Option<MprisPlayerInfo>,
        players: stabby::vec::Vec<MprisPlayerInfo>,
        playback_status: MprisPlaybackStatus,
        metadata: stabby::option::Option<MprisTrackMetadata>,
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

impl TypedMessage for MprisStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_mpris_model::MprisStatusMessage");
}

impl MessageTopic for MprisStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl SharedMessage for MprisStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_STATUS
    }
}
