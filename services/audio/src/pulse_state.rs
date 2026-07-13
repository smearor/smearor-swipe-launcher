/// Tracks the current PulseAudio sink state for command execution.
#[derive(Clone, Debug)]
pub struct PulseState {
    /// Name of the default sink.
    pub default_sink_name: Option<String>,
    /// Index of the default sink.
    pub default_sink_index: Option<u32>,
    /// Current volume ratio (0.0 - 1.5).
    pub volume: f32,
    /// Whether the default sink is muted.
    pub mute: bool,
    /// Number of channels on the default sink.
    pub channels: u8,
    /// Available output sinks: (index, name).
    pub sinks: Vec<(u32, String)>,
    /// Whether a device switch is in progress and pulse_state should not be overwritten by query_status.
    pub pending_switch: bool,
}

impl Default for PulseState {
    fn default() -> Self {
        Self {
            default_sink_name: None,
            default_sink_index: None,
            volume: 0.0,
            mute: false,
            channels: 2,
            sinks: Vec::new(),
            pending_switch: false,
        }
    }
}
