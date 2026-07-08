use serde::Deserialize;
use serde::Serialize;

/// Configuration for a video-based wallpaper theme using `mpvpaper`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct VideoConfig {
    /// Directory containing video files to play.
    pub directory: String,
    /// Target display outputs. `["ALL"]` targets all connected monitors.
    pub outputs: Vec<String>,
    /// Whether to loop the playlist.
    pub loop_playlist: bool,
    /// Whether to shuffle the playlist order.
    pub shuffle: bool,
    /// Whether to mute audio output.
    pub muted: bool,
    /// Audio volume (0-100).
    pub volume: u32,
    /// Playback speed as a percentage (100 = 1.0x normal speed).
    pub speed_percentage: u32,
    /// Additional mpv arguments passed verbatim.
    pub extra_arguments: Vec<String>,
}
