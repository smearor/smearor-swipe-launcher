use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_COMMAND: &str = "service.mpris.command";

/// Actions that can be sent from the widget to the MPRIS service.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
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
    /// Bring the player window to the foreground
    Raise,
    /// Quit the player application
    Quit,
}

/// Command message sent from the MPRIS widget to the MPRIS service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct MprisCommandMessage {
    /// The action to execute
    pub action: MprisCommandAction,
    /// Optional seek offset in microseconds (positive or negative)
    pub seek_offset: stabby::option::Option<i64>,
    /// Optional absolute position in microseconds
    pub position: stabby::option::Option<i64>,
    /// Optional player bus name to target a specific player
    pub player_bus_name: stabby::option::Option<stabby::string::String>,
}

impl MprisCommandMessage {
    pub fn new(
        action: MprisCommandAction,
        seek_offset: stabby::option::Option<i64>,
        position: stabby::option::Option<i64>,
        player_bus_name: stabby::option::Option<stabby::string::String>,
    ) -> Self {
        Self {
            action,
            seek_offset,
            position,
            player_bus_name,
        }
    }

    pub fn play() -> Self {
        Self::new(
            MprisCommandAction::Play,
            stabby::option::Option::None(),
            stabby::option::Option::None(),
            stabby::option::Option::None(),
        )
    }

    pub fn pause() -> Self {
        Self::new(
            MprisCommandAction::Pause,
            stabby::option::Option::None(),
            stabby::option::Option::None(),
            stabby::option::Option::None(),
        )
    }

    pub fn toggle_play_pause() -> Self {
        Self::new(
            MprisCommandAction::TogglePlayPause,
            stabby::option::Option::None(),
            stabby::option::Option::None(),
            stabby::option::Option::None(),
        )
    }

    pub fn stop() -> Self {
        Self::new(
            MprisCommandAction::Stop,
            stabby::option::Option::None(),
            stabby::option::Option::None(),
            stabby::option::Option::None(),
        )
    }

    pub fn next_track() -> Self {
        Self::new(
            MprisCommandAction::NextTrack,
            stabby::option::Option::None(),
            stabby::option::Option::None(),
            stabby::option::Option::None(),
        )
    }

    pub fn previous_track() -> Self {
        Self::new(
            MprisCommandAction::PreviousTrack,
            stabby::option::Option::None(),
            stabby::option::Option::None(),
            stabby::option::Option::None(),
        )
    }

    pub fn seek(offset: i64) -> Self {
        Self::new(
            MprisCommandAction::Seek,
            stabby::option::Option::Some(offset),
            stabby::option::Option::None(),
            stabby::option::Option::None(),
        )
    }

    pub fn set_position(position: i64) -> Self {
        Self::new(
            MprisCommandAction::SetPosition,
            stabby::option::Option::None(),
            stabby::option::Option::Some(position),
            stabby::option::Option::None(),
        )
    }

    pub fn cycle_loop() -> Self {
        Self::new(
            MprisCommandAction::CycleLoop,
            stabby::option::Option::None(),
            stabby::option::Option::None(),
            stabby::option::Option::None(),
        )
    }

    pub fn toggle_shuffle() -> Self {
        Self::new(
            MprisCommandAction::ToggleShuffle,
            stabby::option::Option::None(),
            stabby::option::Option::None(),
            stabby::option::Option::None(),
        )
    }

    pub fn next_player() -> Self {
        Self::new(
            MprisCommandAction::NextPlayer,
            stabby::option::Option::None(),
            stabby::option::Option::None(),
            stabby::option::Option::None(),
        )
    }

    pub fn previous_player() -> Self {
        Self::new(
            MprisCommandAction::PreviousPlayer,
            stabby::option::Option::None(),
            stabby::option::Option::None(),
            stabby::option::Option::None(),
        )
    }

    pub fn raise() -> Self {
        Self::new(
            MprisCommandAction::Raise,
            stabby::option::Option::None(),
            stabby::option::Option::None(),
            stabby::option::Option::None(),
        )
    }

    pub fn quit() -> Self {
        Self::new(
            MprisCommandAction::Quit,
            stabby::option::Option::None(),
            stabby::option::Option::None(),
            stabby::option::Option::None(),
        )
    }
}

impl TypedMessage for MprisCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_mpris_model::MprisCommandMessage");
}

impl MessageTopic for MprisCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl SharedMessage for MprisCommandMessage {
    fn topic(&self) -> &'static str {
        TOPIC_COMMAND
    }
}
