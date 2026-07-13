/// Internal commands sent from the service to the MPRIS async runtime.
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
    Raise,
    Quit,
    RefreshStatus,
}
