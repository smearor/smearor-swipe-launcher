/// Internal commands sent from the service to the PulseAudio async runtime.
#[derive(Debug)]
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
    /// Duck the master volume to the given target ratio (0.0–1.0).
    /// Stores the pre-duck volume so it can be restored later.
    DuckVolume(f32),
    /// Restore the master volume with a linear fade ramp over `ramp_ms` milliseconds
    /// to the given target ratio (0.0–1.0). Use `ramp_ms: 0` for instant restore.
    FadeRestoreVolume {
        target: f32,
        ramp_ms: u64,
    },
}
