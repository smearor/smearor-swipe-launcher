/// Internal commands sent from the service to the PulseAudio async runtime.
pub enum PulseCommand {
    VolumeUp,
    VolumeDown,
    SetVolume(f32),
    ToggleMute,
    Mute,
    Unmute,
    NextDevice,
    PreviousDevice,
    RefreshStatus,
}
