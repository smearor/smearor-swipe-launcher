use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the MPRIS service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MprisMcpTools {
    /// Start playback.
    Play,
    /// Pause playback.
    Pause,
    /// Toggle play/pause.
    TogglePlayPause,
    /// Stop playback.
    Stop,
    /// Skip to next track.
    NextTrack,
    /// Return to previous track.
    PreviousTrack,
    /// Seek by an offset in microseconds.
    Seek,
    /// Set absolute position in microseconds.
    SetPosition,
    /// Cycle loop mode.
    CycleLoop,
    /// Toggle shuffle.
    ToggleShuffle,
    /// Switch to next player.
    NextPlayer,
    /// Switch to previous player.
    PreviousPlayer,
    /// Raise the player window.
    Raise,
    /// Quit the player application.
    Quit,
    /// Refresh player status.
    RefreshStatus,
}

impl AsRef<str> for MprisMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::Play => "mpris_play",
            Self::Pause => "mpris_pause",
            Self::TogglePlayPause => "mpris_toggle_play_pause",
            Self::Stop => "mpris_stop",
            Self::NextTrack => "mpris_next_track",
            Self::PreviousTrack => "mpris_previous_track",
            Self::Seek => "mpris_seek",
            Self::SetPosition => "mpris_set_position",
            Self::CycleLoop => "mpris_cycle_loop",
            Self::ToggleShuffle => "mpris_toggle_shuffle",
            Self::NextPlayer => "mpris_next_player",
            Self::PreviousPlayer => "mpris_previous_player",
            Self::Raise => "mpris_raise",
            Self::Quit => "mpris_quit",
            Self::RefreshStatus => "mpris_refresh_status",
        }
    }
}

impl FromStr for MprisMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "mpris_play" => Ok(Self::Play),
            "mpris_pause" => Ok(Self::Pause),
            "mpris_toggle_play_pause" => Ok(Self::TogglePlayPause),
            "mpris_stop" => Ok(Self::Stop),
            "mpris_next_track" => Ok(Self::NextTrack),
            "mpris_previous_track" => Ok(Self::PreviousTrack),
            "mpris_seek" => Ok(Self::Seek),
            "mpris_set_position" => Ok(Self::SetPosition),
            "mpris_cycle_loop" => Ok(Self::CycleLoop),
            "mpris_toggle_shuffle" => Ok(Self::ToggleShuffle),
            "mpris_next_player" => Ok(Self::NextPlayer),
            "mpris_previous_player" => Ok(Self::PreviousPlayer),
            "mpris_raise" => Ok(Self::Raise),
            "mpris_quit" => Ok(Self::Quit),
            "mpris_refresh_status" => Ok(Self::RefreshStatus),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for MprisMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
